// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

pub(crate) mod client;
pub(crate) mod connection;
pub(crate) mod marshal;

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use client::Client;

use crate::{context::WeakContext, debug, default_topic, log, properties::Properties, refcounted};

default_topic!(log::topic::PROTOCOL);

refcounted! {
    pub(crate) struct Protocol {
        context: RefCell<Option<WeakContext>>,
        name: String,
    }
}

impl Protocol {
    pub(crate) fn new(name: &str) -> Self {
        debug!("Creating new protocol object");
        Self {
            inner: Rc::new(InnerProtocol::new(name)),
        }
    }

    pub(crate) fn set_context(&self, context: WeakContext) {
        self.inner.set_context(context);
    }

    pub(crate) fn new_client(&self, _props: Option<&Properties>) -> Client {
        Client::new()
    }
}

impl InnerProtocol {
    fn new(name: &str) -> Self {
        Self {
            context: RefCell::new(None),
            name: name.to_owned(),
        }
    }

    fn set_context(&self, context: WeakContext) {
        self.context.borrow_mut().replace(context);
    }
}
