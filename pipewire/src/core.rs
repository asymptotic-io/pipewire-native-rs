// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    context::{Context, WeakContext},
    debug, default_topic, log,
    properties::Properties,
    protocol::client::Client,
    proxy::Proxy,
    refcounted,
};

default_topic!(log::topic::CORE);

refcounted! {
    pub struct Core {
        context: WeakContext,
        properties: Properties,
        client: Client,
        proxy: RefCell<Option<Proxy<Core>>>,
    }
}

impl Core {
    pub(crate) fn new(context: &Context, properties: Properties) -> Self {
        let this = Self {
            inner: Rc::new(InnerCore::new(context, properties)),
        };

        this.inner.proxy.borrow_mut().replace(Proxy::new(&this));

        this
    }
}

impl InnerCore {
    fn new(context: &Context, mut properties: Properties) -> Self {
        debug!("Creating new core");

        properties.add_dict(&context.properties().dict());

        // TODO: Create mempool

        let client = context.protocol().new_client(None);

        // TODO: Create proxy for core and client

        Self {
            context: context.downgrade(),
            properties,
            client,
            proxy: RefCell::new(None),
        }
    }
}
