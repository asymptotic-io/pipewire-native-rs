// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    os::fd::RawFd,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

use bitflags::bitflags;
use pipewire_native_spa as spa;

use crate::{
    context::{Context, WeakContext},
    debug, default_topic,
    id_map::IdMap,
    keys, log,
    properties::Properties,
    protocol,
    proxy::{self, HasProxy, Proxy, ProxyEvents},
    refcounted, some_closure, types, Id, Refcounted,
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
        hooks: Arc<Mutex<spa::hook::HookList<CoreEvents>>>,
    }
}

impl Core {
    pub(crate) fn new(context: &Context, properties: Properties) -> std::io::Result<Self> {
        let this = Self {
            inner: Rc::new(InnerCore::new(context, properties)),
        };

        // Reserve id 0 because we are id 0
        let _ = this.inner.proxies.borrow_mut().reserve();
        let core_proxy = Proxy::new_weak(0, &this);
        this.inner.proxy.borrow_mut().replace(core_proxy.clone());

        let id = this.inner.proxies.borrow_mut().reserve();
        let client = proxy::client::Client::new(id);
        this.inner
            .proxies
            .borrow_mut()
            .insert_at(id, Box::new(client));

        this.inner.client.set_core(this.downgrade());

        core_proxy.add_listener(ProxyEvents {
            destroy: some_closure!(this, {
                todo!("clean up proxies etc., or delegate to Drop");
            }),
            bound: None,
            removed: None,
            done: None,
            error: None,
            bound_props: None,
        });

        // add core event listeners
        // send hello
        // update client properties

        this.inner
            .client
            .connect(Some(&this.inner.properties.dict()), None)?;

        Ok(this)
    }

    pub(crate) fn context(&self) -> Context {
        self.inner
            .context
            .upgrade()
            .expect("Context should outlive core")
    }

    pub fn add_listener(&self, events: CoreEvents) {
        self.inner.hooks.lock().unwrap().append(events);
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

bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CoreChangeMask : u32 {
        const PROPS = (1 << 0);
    }
}

pub struct CoreInfo<'a> {
    pub id: u32,
    pub cookie: u32,
    pub user_name: &'a str,
    pub host_name: &'a str,
    pub version: &'a str,
    pub name: &'a str,
    pub mask: CoreChangeMask,
    pub props: &'a spa::dict::Dict,
}

pub struct CoreEvents {
    pub info: Option<Box<dyn FnMut(CoreInfo<'_>)>>,
    pub done: Option<Box<dyn FnMut(u32, u32)>>,
    pub error: Option<Box<dyn FnMut(u32, u32, u32, &str)>>,
    pub(crate) ping: Option<Box<dyn FnMut(u32, u32)>>,
    pub(crate) remove_id: Option<Box<dyn FnMut(u32)>>,
    pub(crate) bound_id: Option<Box<dyn FnMut(u32, u32)>>,
    pub(crate) add_mem: Option<Box<dyn FnMut(u32, u32, RawFd, u32)>>,
    pub(crate) remove_mem: Option<Box<dyn FnMut(u32)>>,
    pub(crate) bound_props: Option<Box<dyn FnMut(u32, u32, &spa::dict::Dict)>>,
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
            hooks: spa::hook::HookList::new(),
        }
    }
}
