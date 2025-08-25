// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

pub(crate) mod client;
pub(crate) mod connection;
pub(crate) mod marshal;

use std::sync::RwLock;

use client::Client;

use crate::{
    context::WeakContext, debug, default_topic, log, new_refcounted, properties::Properties,
    refcounted,
};

default_topic!(log::topic::PROTOCOL);

const ASYNC_SEQ_BIT: u32 = 1 << 30;
const ASYNC_SEQ_MASK: u32 = ASYNC_SEQ_BIT - 1;

refcounted! {
    pub(crate) struct Protocol {
        context: RwLock<Option<WeakContext>>,
        #[allow(unused)]
        name: String,
    }
}

impl Protocol {
    pub(crate) fn new(name: &str) -> Self {
        debug!("Creating new protocol object");
        Self {
            inner: new_refcounted(InnerProtocol::new(name)),
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
            context: RwLock::new(None),
            name: name.to_owned(),
        }
    }

    fn set_context(&self, context: WeakContext) {
        self.context.write().unwrap().replace(context);
    }
}
