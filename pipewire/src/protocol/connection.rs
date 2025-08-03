// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    io::Write,
    os::{fd::RawFd, unix::net::UnixStream},
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

use pipewire_native_spa::{self as spa, pod::Pod};

use crate::{debug, default_topic, log, protocol::marshal, refcounted, Id};

default_topic!(log::topic::CONNECTION);

const MAX_MESSAGE_SIZE: usize = 16_777_216;

refcounted! {
    pub(crate) struct Connection {
        stream: RefCell<Option<UnixStream>>,
        hooks: Arc<Mutex<spa::hook::HookList<ConnectionEvents>>>,
        // Data to send
        seq: RefCell<u32>,
        buf: RefCell<Vec<u8>>,
        fds: RefCell<Vec<RawFd>>,
    }
}

pub(crate) struct ConnectionEvents {
    pub(crate) destroy: Option<Box<dyn FnMut()>>,
    pub(crate) error: Option<Box<dyn FnMut(u32)>>,
    pub(crate) need_flush: Option<Box<dyn FnMut()>>,
    pub(crate) start: Option<Box<dyn FnMut(u32)>>,
}

impl Connection {
    pub(crate) fn new(stream: Option<UnixStream>) -> Self {
        debug!("Creating new connection to {stream:?}");
        Self {
            inner: Rc::new(InnerConnection::new(stream)),
        }
    }

    pub(crate) fn set_stream(&self, stream: UnixStream) {
        self.inner.stream.replace(Some(stream));
    }

    pub(crate) fn add_listener(&self, events: ConnectionEvents) -> spa::hook::HookId {
        self.inner.hooks.lock().unwrap().append(events)
    }

    pub(crate) fn remove_listener(&self, listener: spa::hook::HookId) {
        let _ = self.inner.hooks.lock().unwrap().remove(listener);
    }

    pub(crate) fn push<T: spa::pod::Pod<DecodesTo = T>>(
        &self,
        id: Id,
        opcode: u8,
        data: T,
    ) -> std::io::Result<()> {
        let seq = *self.inner.seq.borrow();
        let message = marshal::Message {
            header: marshal::Header {
                id,
                opcode,
                seq,
                size: 0,  // filled by encode
                n_fds: 0, // TOOO
            },
            object: data,
            footer: None, // TODO
        };

        loop {
            let mut buf = self.inner.buf.borrow_mut();
            let rest = unsafe { std::mem::transmute(buf.spare_capacity_mut()) };

            match message.encode(rest) {
                Ok(_) => break,
                Err(spa::pod::Error::NoSpace) => {
                    let capacity = buf.capacity();
                    if capacity > MAX_MESSAGE_SIZE {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("cannot send message > {MAX_MESSAGE_SIZE}"),
                        ));
                    }
                    buf.reserve(capacity);
                    // And now we try again
                }
                _ => unreachable!(),
            }
        }

        self.inner.seq.replace(seq + 1);
        spa::emit_hook!(self.inner.hooks, need_flush);

        Ok(())
    }

    pub(crate) fn flush(&self) -> std::io::Result<()> {
        let mut o_stream = self.inner.stream.borrow_mut();
        let stream = o_stream.as_mut().unwrap();
        let mut buf = self.inner.buf.borrow_mut();
        let mut idx = 0;
        let mut res = Ok(());

        while idx < buf.len() {
            let sent = match stream.write(&buf[idx..]) {
                Ok(size) => size,
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::Interrupted {
                        continue;
                    } else {
                        res = Err(err);
                        break;
                    }
                }
            };

            idx += sent;
        }

        if idx == buf.len() {
            buf.clear();
        } else {
            buf.copy_within(idx.., 0);
        }

        res
    }
}

impl InnerConnection {
    pub(crate) fn new(stream: Option<UnixStream>) -> Self {
        InnerConnection {
            stream: RefCell::new(stream),
            hooks: spa::hook::HookList::new(),
            seq: RefCell::new(0),
            buf: RefCell::new(Vec::with_capacity(16384)), // Initial size, can grow if needed
            fds: RefCell::new(Vec::new()),
        }
    }
}
