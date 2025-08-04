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

use crate::{
    debug, default_topic, log,
    protocol::{marshal, ASYNC_SEQ_MASK},
    refcounted, trace, Id,
};

use super::marshal::Marshallable;

default_topic!(log::topic::CONNECTION);

const MAX_MESSAGE_SIZE: usize = 16_777_216;

refcounted! {
    pub(crate) struct Connection {
        stream: RefCell<Option<UnixStream>>,
        hooks: Arc<Mutex<spa::hook::HookList<ConnectionEvents>>>,
        // Data to send
        out_seq: RefCell<u32>,
        out_buf: RefCell<Vec<u8>>,
        out_size: RefCell<usize>,
        out_fds: RefCell<Vec<RawFd>>,
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

    pub(crate) fn next_seq(&self) -> u32 {
        *self.inner.out_seq.borrow()
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

    pub(crate) fn push<T: Marshallable>(&self, id: Id, object: T) -> std::io::Result<()> {
        let seq = *self.inner.out_seq.borrow();
        let message = marshal::Message {
            header: marshal::Header {
                id,
                opcode: object.opcode(),
                seq,
                size: 0,  // filled by encode
                n_fds: 0, // TOOO
            },
            object,
            footer: None, // TODO
        };

        trace!("pushing message id:{id} opcode:{}", message.header.opcode);

        let mut buf = self.inner.out_buf.borrow_mut();
        let mut size = self.inner.out_size.borrow_mut();

        loop {
            let rest = &mut buf.as_mut_slice()[*size..];
            match message.encode(rest) {
                Ok(written) => {
                    *size += written;
                    break;
                }
                Err(spa::pod::Error::NoSpace) => {
                    let capacity = buf.capacity();
                    if capacity > MAX_MESSAGE_SIZE {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidInput,
                            format!("cannot send message > {MAX_MESSAGE_SIZE}"),
                        ));
                    }
                    buf.resize(capacity * 2, 0);
                    // And now we try again
                }
                _ => unreachable!(),
            }
        }

        self.inner.out_seq.replace((seq + 1) & ASYNC_SEQ_MASK);
        spa::emit_hook!(self.inner.hooks, need_flush);

        Ok(())
    }

    pub(crate) fn flush(&self) -> std::io::Result<()> {
        let mut o_stream = self.inner.stream.borrow_mut();
        let stream = o_stream.as_mut().unwrap();
        let mut buf = self.inner.out_buf.borrow_mut();
        let mut size = self.inner.out_size.borrow_mut();
        let mut idx = 0;
        let mut res = Ok(());

        trace!("flushing {} bytes", *size);

        while idx < *size {
            let sent = match stream.write(&buf[idx..*size]) {
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
            *size = 0;
        } else {
            buf.copy_within(idx.., 0);
            *size -= idx;
        }

        res
    }
}

impl InnerConnection {
    pub(crate) fn new(stream: Option<UnixStream>) -> Self {
        InnerConnection {
            stream: RefCell::new(stream),
            hooks: spa::hook::HookList::new(),
            out_seq: RefCell::new(0),
            out_buf: RefCell::new(vec![0; 16384]),
            out_size: RefCell::new(0),
            out_fds: RefCell::new(Vec::new()),
        }
    }
}
