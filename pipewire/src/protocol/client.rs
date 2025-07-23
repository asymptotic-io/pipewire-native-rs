// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    core::WeakCore, debug, default_topic, log, protocol::connection::Connection, refcounted,
};

default_topic!(log::topic::PROTOCOL);

refcounted! {
    pub(crate) struct Client {
        core: RefCell<Option<WeakCore>>,
        connection: Connection,
    }
}

impl Client {
    pub(crate) fn new() -> Self {
        debug!("Creating new client");
        Self {
            inner: Rc::new(InnerClient::new()),
        }
    }

    pub(crate) fn set_core(&self, core: WeakCore) {
        self.inner.set_core(core);
    }
}

impl InnerClient {
    fn new() -> Self {
        Self {
            core: RefCell::new(None),
            connection: Connection::new(-1),
        }
    }

    fn set_core(&self, core: WeakCore) {
        self.core.borrow_mut().replace(core);
    }
}
