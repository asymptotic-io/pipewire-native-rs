// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    os::fd::RawFd,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

use pipewire_native_spa as spa;

use crate::{debug, default_topic, log, refcounted};

default_topic!(log::topic::PROTOCOL);

refcounted! {
    pub(crate) struct Connection {
        fd: RefCell<RawFd>,
        hooks: Arc<Mutex<spa::hook::HookList<ConnectionEvents>>>,
    }
}

pub(crate) struct ConnectionEvents {
    pub(crate) destroy: Option<Box<dyn FnMut()>>,
    pub(crate) error: Option<Box<dyn FnMut(u32)>>,
    pub(crate) need_flush: Option<Box<dyn FnMut()>>,
    pub(crate) start: Option<Box<dyn FnMut(u32)>>,
}

impl Connection {
    pub(crate) fn new(fd: RawFd) -> Self {
        debug!("Creating new connection to {fd}");
        Self {
            inner: Rc::new(InnerConnection::new(fd)),
        }
    }

    pub(crate) fn set_fd(&self, fd: RawFd) {
        self.inner.fd.replace(fd);
    }

    pub(crate) fn add_listener(&self, events: ConnectionEvents) -> spa::hook::HookId {
        self.inner.hooks.lock().unwrap().append(events)
    }

    pub(crate) fn remove_listener(&self, listener: spa::hook::HookId) {
        let _ = self.inner.hooks.lock().unwrap().remove(listener);
    }
}

impl InnerConnection {
    pub(crate) fn new(fd: RawFd) -> Self {
        InnerConnection {
            fd: RefCell::new(fd),
            hooks: spa::hook::HookList::new(),
        }
    }
}
