// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use pipewire_native_spa as spa;

use crate::{
    context::{Context, WeakContext},
    debug, default_topic,
    id_map::IdMap,
    keys, log,
    properties::Properties,
    protocol,
    proxy::{self, HasProxy, Proxy},
    refcounted, types,
};

default_topic!(log::topic::CORE);

pub const DEFAULT_REMOTE: &str = "pipewire-0";

pub(crate) fn get_remote(props: Option<&spa::dict::Dict>) -> String {
    std::env::var("PIPEWIRE_REMOTE")
        .ok()
        .filter(|v| v.len() > 0)
        .or_else(|| {
            props
                .and_then(|p| p.lookup(keys::REMOTE_NAME).to_owned())
                .filter(|v| v.len() > 0)
                .map(|s| s.to_owned())
        })
        .unwrap_or(DEFAULT_REMOTE.to_owned())
}

refcounted! {
    pub struct Core {
        context: WeakContext,
        properties: Properties,
        client: protocol::client::Client,
        proxy: RefCell<Option<Proxy<Core>>>,
        proxies: RefCell<IdMap<Box<dyn HasProxy>>>,
    }
}

impl Core {
    pub(crate) fn new(context: &Context, properties: Properties) -> std::io::Result<Self> {
        let this = Self {
            inner: Rc::new(InnerCore::new(context, properties)),
        };

        // Reserve id 0 because we are id 0
        let _ = this.inner.proxies.borrow_mut().reserve();
        this.inner
            .proxy
            .borrow_mut()
            .replace(Proxy::new_weak(0, &this));

        let id = this.inner.proxies.borrow_mut().reserve();
        let client = proxy::client::Client::new(id);
        this.inner
            .proxies
            .borrow_mut()
            .insert_at(id, Box::new(client));

        // add core and proxy event listeners
        // send hello
        // update client properties

        this.inner
            .client
            .connect(Some(&this.inner.properties.dict()), None)?;

        Ok(this)
    }
}

impl HasProxy for Core {
    fn type_() -> types::ObjectType {
        types::interface::CORE
    }

    fn version() -> u32 {
        4
    }

    fn proxy(&self) -> Proxy<Self> {
        self.inner
            .proxy
            .borrow()
            .as_ref()
            .expect("Proxy should be initialised")
            .clone()
    }
}

impl InnerCore {
    fn new(context: &Context, mut properties: Properties) -> Self {
        debug!("Creating new core");

        properties.add_dict(&context.properties().dict());

        // TODO: Create mempool

        let client = context.protocol().new_client(None);

        Self {
            context: context.downgrade(),
            properties,
            client,
            proxy: RefCell::new(None),
            proxies: RefCell::new(IdMap::new()),
        }
    }
}
