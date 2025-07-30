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
    rc::{Rc, Weak},
};

use pipewire_native_spa as spa;

use crate::{
    core::{self, WeakCore},
    debug, default_topic, keys, log,
    protocol::connection::{Connection, ConnectionEvents},
    refcounted,
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
        connection: Connection,
        listener: RefCell<Option<spa::hook::HookId>>,
    }
}

impl Client {
    pub(crate) fn new() -> Self {
        debug!("Creating new client");
        let this = Self {
            inner: Rc::new(InnerClient::new()),
        };

        let weak_client = this.downgrade();
        let listener = this.inner.connection.add_listener(ConnectionEvents {
            destroy: Some(Box::new(move || {
                let client = weak_client
                    .upgrade()
                    .expect("Client should outlive connection");
                client.on_destroy();
            })),
            error: None,
            need_flush: Some(Box::new(|| todo!("implement client need_flush"))),
            start: None,
        });

        this.inner.listener.borrow_mut().replace(listener);

        this
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

    pub(crate) fn set_fd(&self, fd: RawFd, close: bool) -> std::io::Result<()> {
        self.inner.connection.set_fd(fd);
        // hook up source to process messages

        Ok(())
    }

    fn on_destroy(&self) {
        self.inner
            .connection
            .remove_listener(self.inner.listener.borrow().unwrap());
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

        let res = self.set_fd(stream.as_raw_fd(), true);

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
            connection: Connection::new(-1),
            listener: RefCell::new(None),
        }
    }

    fn set_core(&self, core: WeakCore) {
        self.core.borrow_mut().replace(core);
    }
}
