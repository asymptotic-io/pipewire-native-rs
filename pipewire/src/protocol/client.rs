// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    os::{
        fd::{AsRawFd, RawFd},
        unix::net::UnixStream,
    },
    path::PathBuf,
    pin::Pin,
    rc::{Rc, Weak},
};

use pipewire_native_spa as spa;

use crate::{
    closure,
    core::{self, Core, WeakCore},
    debug, default_topic, keys, log,
    protocol::connection::{Connection, ConnectionEvents},
    proxy::HasProxy,
    proxy_notify, refcounted, some_closure, trace, warn,
};

default_topic!(log::topic::PROTOCOL);

fn get_runtime_dir() -> Option<String> {
    std::env::var("PIPEWIRE_RUNTIME_DIR")
        .or(std::env::var("XDG_RUNTIME_DIR"))
        .or(std::env::var("USERPROFILEDIR"))
        .ok()
}

fn get_system_dir() -> String {
    "/run/pipewire".to_owned()
}

refcounted! {
    pub(crate) struct Client {
        core: RefCell<Option<WeakCore>>,
        stream: RefCell<Option<UnixStream>>,
        connection: Connection,
        connected: RefCell<bool>,
        need_flush: RefCell<bool>,
        source: RefCell<Option<Pin<Box<spa::interface::r#loop::LoopUtilsSource>>>>,
        listener: RefCell<Option<spa::hook::HookId>>,
    }
}

impl Client {
    pub(crate) fn new() -> Self {
        debug!("Creating new client");
        let this = Self {
            inner: Rc::new(InnerClient::new()),
        };

        let listener = this.inner.connection.add_listener(ConnectionEvents {
            destroy: some_closure!(this, {
                this.on_destroy();
            }),
            error: None,
            need_flush: some_closure!(this, {
                this.on_need_flush();
            }),
            start: None,
        });

        this.inner.listener.borrow_mut().replace(listener);

        this
    }

    pub(crate) fn connection(&self) -> Connection {
        self.inner.connection.clone()
    }

    pub(crate) fn core(&self) -> Core {
        self.inner
            .core
            .borrow()
            .clone()
            .and_then(|w| w.upgrade())
            .expect("Client shoud have core initialised on creation")
    }

    pub(crate) fn set_core(&self, core: WeakCore) {
        self.inner.set_core(core);
    }

    pub(crate) fn connect(
        &self,
        props: Option<&spa::dict::Dict>,
        done_cb: Option<Box<dyn Fn(std::io::Result<()>)>>,
    ) -> std::io::Result<()> {
        // TODO: Implement PW_KEY_REMOTE_INTENTION != "generic" (i.e. screencast and internal remotes)
        self.connect_local_socket(props, done_cb)
    }

    pub(crate) fn set_stream(&self, stream: UnixStream, close: bool) -> std::io::Result<()> {
        debug!("Setting fd on connection: {stream:?}");

        let fd = stream.as_raw_fd();

        self.inner
            .connection
            .set_stream(stream.try_clone().expect("unix stream should be cloneable"));
        self.inner.stream.replace(Some(stream));
        self.inner.connected.replace(true);

        let main_loop = self.core().context().main_loop();

        let source = main_loop.add_io(
            fd,
            spa::flags::Io::all(),
            close,
            closure!(client <- self, fd, mask, {
                client.on_remote_data(fd, spa::flags::Io::from_bits_truncate(mask));
            }),
        );

        self.inner.source.replace(source);

        Ok(())
    }

    fn on_destroy(&self) {
        self.inner
            .connection
            .remove_listener(self.inner.listener.borrow().unwrap());
    }

    fn on_need_flush(&self) {
        self.inner.need_flush.replace(true);

        if let Some(source) = self.inner.source.borrow_mut().as_mut() {
            let main_loop = self.core().context().main_loop();
            let _ = main_loop.update_io(source, source.mask | spa::flags::Io::OUT);
        }
    }

    fn on_remote_data(&self, _fd: RawFd, mask: spa::flags::Io) {
        trace!("on remote data: {mask:?}");

        if mask.intersects(spa::flags::Io::ERR | spa::flags::Io::HUP) {
            self.on_connection_error(
                std::io::Error::from(std::io::ErrorKind::BrokenPipe),
                "I/O error",
            );
            return;
        }

        if mask.contains(spa::flags::Io::IN) {
            trace!("incoming data");
        }

        if mask.contains(spa::flags::Io::OUT) || *self.inner.need_flush.borrow() {
            self.inner.need_flush.replace(false);

            match self.inner.stream.borrow().as_ref().unwrap().take_error() {
                Ok(None) => { /* all good, nothing to do */ }
                Ok(Some(err)) => {
                    self.on_connection_error(err, "connection error");
                    return;
                }
                Err(err) => {
                    self.on_connection_error(err, "getsockopt failed");
                    return;
                }
            }

            match self.inner.connection.flush() {
                Ok(_) => {
                    let main_loop = self.core().context().main_loop();
                    let mut source_ref = self.inner.source.borrow_mut();
                    let source = source_ref.as_mut().unwrap();
                    let _ = main_loop.update_io(source, source.mask & !spa::flags::Io::OUT);
                }
                Err(err) => {
                    if err.raw_os_error() != Some(libc::EAGAIN) {
                        self.on_connection_error(err, "flush failed");
                    }
                }
            }
        }
    }

    fn on_connection_error(&self, err: std::io::Error, msg: &str) {
        warn!("Got connection error: {:?}", err);

        if let Some(source) = self.inner.source.take() {
            let main_loop = self.core().context().main_loop();
            main_loop.destroy_source(source);
        }

        let core = &self.core();
        let res = err.raw_os_error().unwrap_or(err.kind() as i32).abs() as u32;

        proxy_notify!(core, error, 0 /* TODO: recv_seq */, res, msg);
    }

    fn connect_local_socket(
        &self,
        props: Option<&spa::dict::Dict>,
        done_cb: Option<Box<dyn Fn(std::io::Result<()>)>>,
    ) -> std::io::Result<()> {
        let manager = props.and_then(|p| p.lookup(keys::REMOTE_INTENTION)) == Some("manager");
        let mut remote_name = core::get_remote(props);

        // TODO: remote can be a list of remotes

        if manager && !remote_name.ends_with("-manager") {
            remote_name = format!("{remote_name}-manager");
        }

        if remote_name.starts_with("/") || remote_name.starts_with("@") {
            // Absolute path
            self.try_connect_local_socket(None, &remote_name, done_cb.as_ref())
        } else {
            // Relative path
            if let Some(runtime_dir) = get_runtime_dir() {
                if self
                    .try_connect_local_socket(Some(&runtime_dir), &remote_name, done_cb.as_ref())
                    .is_ok()
                {
                    // Connect via runtime dir worked
                    return Ok(());
                }
            }

            // Fallback to connect via system dir
            self.try_connect_local_socket(Some(&get_system_dir()), &remote_name, done_cb.as_ref())
        }
    }

    fn try_connect_local_socket(
        &self,
        path: Option<&str>,
        name: &str,
        done_cb: Option<&Box<dyn Fn(std::io::Result<()>)>>,
    ) -> std::io::Result<()> {
        let mut socket_path = PathBuf::new();

        if let Some(path) = path {
            socket_path.push(path);
        }

        socket_path.push(name);

        debug!("Trying to connect to {:?}", socket_path);

        // Rust sockets are implicitly CLOEXEC
        let stream = UnixStream::connect(socket_path)?;
        stream.set_nonblocking(true)?;

        let res = self.set_stream(stream, true);

        if let Some(cb) = done_cb {
            cb(res);
        }

        Ok(())
    }
}

impl InnerClient {
    fn new() -> Self {
        Self {
            core: RefCell::new(None),
            stream: RefCell::new(None),
            connection: Connection::new(None),
            connected: RefCell::new(false),
            need_flush: RefCell::new(false),
            source: RefCell::new(None),
            listener: RefCell::new(None),
        }
    }

    fn set_core(&self, core: WeakCore) {
        self.core.borrow_mut().replace(core);
    }
}
