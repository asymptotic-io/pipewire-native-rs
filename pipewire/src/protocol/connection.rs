// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    io::{Read, Write},
    os::{fd::RawFd, unix::net::UnixStream},
    sync::{Arc, Mutex},
};

use pipewire_native_spa::{self as spa, pod::Pod};

use crate::{
    debug, default_topic, log, new_refcounted, protocol::ASYNC_SEQ_MASK, refcounted, trace, Id,
};

use super::marshal::{
    self,
    message::{ClientFooter, Header, Message},
    message::{ClientFooterPayload, ClientGeneration, CoreFooter, CoreFooterPayload},
    Marshallable,
};

default_topic!(log::topic::CONNECTION);

const MAX_MESSAGE_SIZE: usize = 16_777_216;

refcounted! {
    pub(crate) struct Connection {
        stream: RefCell<Option<UnixStream>>,
        hooks: Arc<Mutex<spa::hook::HookList<ConnectionEvents>>>,
        // Data received
        in_buf: RefCell<Vec<u8>>,
        in_size: RefCell<usize>,
        in_offset: RefCell<usize>,
        last_recv_generation: RefCell<i64>,
        // Data to send
        out_seq: RefCell<u32>,
        out_buf: RefCell<Vec<u8>>,
        out_size: RefCell<usize>,
        out_fds: RefCell<Vec<RawFd>>,
        last_sent_generation: RefCell<i64>,
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
            inner: new_refcounted(InnerConnection::new(stream)),
        }
    }

    pub(crate) fn next_seq(&self) -> u32 {
        *self.inner.out_seq.borrow()
    }

    pub(crate) fn set_stream(&self, stream: UnixStream) {
        self.inner.stream.replace(Some(stream));
    }

    pub(crate) fn disconnect(&self) {
        self.inner.stream.replace(None);
        self.clear_buffers();
    }

    fn clear_buffers(&self) {
        self.inner.in_buf.borrow_mut().fill(0);
        self.inner.in_size.replace(0);
        self.inner.in_offset.replace(0);
        self.inner.last_recv_generation.replace(0);
        self.inner.out_seq.replace(0);
        self.inner.out_buf.borrow_mut().fill(0);
        self.inner.out_size.replace(0);
        self.inner.out_fds.borrow_mut().clear();
        self.inner.last_sent_generation.replace(0);
    }

    pub(crate) fn add_listener(&self, events: ConnectionEvents) -> spa::hook::HookId {
        self.inner.hooks.lock().unwrap().append(events)
    }

    pub(crate) fn remove_listener(&self, listener: spa::hook::HookId) {
        let _ = self.inner.hooks.lock().unwrap().remove(listener);
    }

    pub(crate) fn push<T: Marshallable + std::fmt::Debug>(
        &self,
        id: Id,
        object: T,
    ) -> std::io::Result<()> {
        let seq = *self.inner.out_seq.borrow();

        let recv_generation = self.inner.last_recv_generation.borrow();
        let mut sent_generation = self.inner.last_sent_generation.borrow_mut();

        // TODO: support CoreGeneration as well when we implement server
        let footer = if *recv_generation > *sent_generation {
            *sent_generation = *recv_generation;
            trace!("sending client generation {}", *recv_generation);
            let mut footer = ClientFooter::new();
            footer.push(ClientFooterPayload::Generation(ClientGeneration {
                client_generation: *recv_generation,
            }));
            Some(footer)
        } else {
            None
        };

        let message = Message {
            header: Header {
                id,
                opcode: object.opcode(),
                seq,
                size: 0,  // filled by encode
                n_fds: 0, // TOOO
            },
            object,
            footer,
        };

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
                    let capacity = buf.len();
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

        trace!(
            "pushed message id:{id} opcode:{} seq:{seq} payload:{:?} (filled: {size})",
            message.header.opcode,
            message.object
        );

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

    pub(crate) fn next_message(&self) -> std::io::Result<Header> {
        loop {
            let (wanted_capacity, header) = self.parse_next()?;
            trace!(
                "we need {wanted_capacity}, got header: {}",
                header.is_some()
            );

            if self.inner.in_buf.borrow().len() < wanted_capacity {
                // Not enough space for header or message, make some space, try to fill some data,
                // and then retry
                trace!(
                    "expanding capacity from {} -> {}",
                    self.inner.in_buf.borrow().len(),
                    wanted_capacity
                );
                self.inner.in_buf.borrow_mut().resize(wanted_capacity, 0);
                self.read()?;
            } else if let Some(header) = header {
                // We had enough space, and got the header, so we should be good to have the caller
                // try to decode the message too
                trace!(
                    "got message id:{} opcode:{} seq:{} size:{}",
                    header.id,
                    header.opcode,
                    header.seq,
                    header.size
                );
                return Ok(header);
            } else {
                // We had enough space, but don't have the data, let's try to read data into the
                // buffer
                self.read()?;
            }
        }
    }

    pub(crate) fn decode_message<T: Marshallable, F: Pod<DecodesTo = F>>(
        &self,
        header: &Header,
    ) -> std::io::Result<(T, Option<F>)> {
        let buf = self.inner.in_buf.borrow_mut();
        let mut size = self.inner.in_size.borrow_mut();
        let mut offset = self.inner.in_offset.borrow_mut();

        let start = *offset + marshal::HEADER_LEN;
        let end = start + header.size as usize;

        let (body, body_size) = T::decode(header.opcode, &buf[start..end]).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Could not decode message body: {e:?}"),
            )
        })?;

        let (footer, footer_size) = if body_size < header.size as usize {
            let (f, fs) = F::decode(&buf[start + body_size..]).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Could not decode message footer: {e:?}"),
                )
            })?;
            (Some(f), fs)
        } else {
            (None, 0)
        };

        if body_size + footer_size != header.size as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!(
                    "Mismatched message size({}) and body size({}) + footer_size({})",
                    header.size, body_size, footer_size
                ),
            ));
        }

        *offset += marshal::HEADER_LEN + header.size as usize;

        if *offset == *size {
            // We've consumed all the data
            *offset = 0;
            *size = 0;
        }

        Ok((body, footer))
    }

    pub(crate) fn decode_core_message<T: Marshallable>(
        &self,
        header: &Header,
    ) -> std::io::Result<T> {
        let (object, footer) = self.decode_message(header)?;

        self.update_generation(footer.as_ref());

        Ok(object)
    }

    // TODO: support CoreGeneration as well when we implement server
    pub fn update_generation(&self, footer: Option<&CoreFooter>) {
        if let Some(footer) = footer {
            for p in &footer.payloads {
                match p {
                    CoreFooterPayload::Generation(g) => {
                        trace!("updating core generation to {}", g.registry_generation);
                        self.inner
                            .last_recv_generation
                            .replace(g.registry_generation);
                    }
                }
            }
        }
    }

    fn parse_next(&self) -> std::io::Result<(usize, Option<Header>)> {
        let size = *self.inner.in_size.borrow();
        let offset = *self.inner.in_offset.borrow();

        if size - offset < marshal::HEADER_LEN {
            return Ok((marshal::HEADER_LEN, None));
        }

        trace!("looking for message header from [{offset}..{size}]");

        let buf = self.inner.in_buf.borrow();
        let header = match Header::decode(&buf[offset..size]) {
            Ok((header, _)) => header,
            Err(e) => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Failed to parse message: {e:?}"),
                ))
            }
        };

        Ok((
            offset + marshal::HEADER_LEN + header.size as usize,
            Some(header),
        ))
    }

    fn read(&self) -> std::io::Result<()> {
        let mut stream_ref = self.inner.stream.borrow_mut();
        let stream = stream_ref.as_mut().unwrap();
        let mut buf = self.inner.in_buf.borrow_mut();
        let mut size = self.inner.in_size.borrow_mut();

        let read = stream.read(&mut buf[*size..])?;
        trace!("read {read} bytes at {size}");

        // TODO: control messages

        if read > 0 {
            *size += read;
            Ok(())
        } else {
            // Nothing to process, we're done
            Err(std::io::Error::from_raw_os_error(libc::EAGAIN))
        }
    }
}

impl InnerConnection {
    pub(crate) fn new(stream: Option<UnixStream>) -> Self {
        InnerConnection {
            stream: RefCell::new(stream),
            hooks: spa::hook::HookList::new(),
            in_buf: RefCell::new(vec![0; 16384]),
            in_size: RefCell::new(0),
            in_offset: RefCell::new(0),
            last_recv_generation: RefCell::new(0),
            out_seq: RefCell::new(0),
            out_buf: RefCell::new(vec![0; 16384]),
            out_size: RefCell::new(0),
            out_fds: RefCell::new(Vec::new()),
            last_sent_generation: RefCell::new(0),
        }
    }
}
