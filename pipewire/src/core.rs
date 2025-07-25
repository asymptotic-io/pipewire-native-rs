// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use crate::{
    context::{Context, WeakContext},
    debug, default_topic,
    id_map::IdMap,
    log,
    properties::Properties,
    protocol::client::Client,
    proxy::Proxy,
    refcounted, types, Id,
};

default_topic!(log::topic::CORE);

refcounted! {
    pub struct Core {
        context: WeakContext,
        properties: Properties,
        client: Client,
        proxies: RefCell<IdMap<Box<dyn Proxy>>>,
    }
}

impl Core {
    pub(crate) fn new(context: &Context, properties: Properties) -> Self {
        let this = Self {
            inner: Rc::new(InnerCore::new(context, properties)),
        };

        // Reserve id 0 because we are id 0
        let _ = this.inner.proxies.borrow_mut().reserve();

        this
    }
}

impl Proxy for Core {
    fn type_() -> types::ObjectType {
        types::interface::CORE
    }

    fn version() -> u32 {
        4
    }

    fn id(&self) -> Id {
        // Can id ever be non-zero for Core?
        0
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
            proxies: RefCell::new(IdMap::new()),
        }
    }
}
