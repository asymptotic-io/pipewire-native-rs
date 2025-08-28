// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::sync::{Arc, Mutex, RwLock};

use bitflags::bitflags;
use pipewire_native_spa as spa;

use crate::{
    core::Core,
    new_refcounted,
    properties::Properties,
    proxy::{HasProxy, Proxy},
    refcounted, types, Id,
};

refcounted! {
    /// Proxy that represents a factory that is loaded on the server.
    pub struct Factory {
        proxy: RwLock<Option<Proxy<Factory>>>,
        hooks: Arc<Mutex<spa::hook::HookList<FactoryEvents>>>,
    }
}

bitflags! {
    /// A bit mask of changes signalled in the [FactoryEvents::info] event.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FactoryChangeMask : u32 {
        /// Factory properties changed.
        const PROPS = (1 << 0);
    }
}

/// Factory information that is provided in a [FactoryEvents::info] event.
pub struct FactoryInfo<'a> {
    /// The ID of the factory.
    pub id: Id,
    /// The name of the factory
    pub name: &'a str,
    /// The type of the factory
    pub type_: &'a str,
    /// The version of the factory
    pub version: u32,
    /// What changed since the last call.
    pub mask: FactoryChangeMask,
    /// The factory's properties.
    pub props: &'a Properties,
}

/// Factory events that can be subscribed to.
#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct FactoryEvents {
    /// Factory information became available, or changed.
    pub info: Option<Box<dyn FnMut(&FactoryInfo<'_>)>>,
}

impl HasProxy for Factory {
    fn type_(&self) -> types::ObjectType {
        types::interface::FACTORY
    }

    fn version(&self) -> u32 {
        3
    }

    fn proxy(&self) -> Proxy<Self> {
        self.inner
            .proxy
            .read()
            .unwrap()
            .as_ref()
            .expect("Factory proxy should be initialised on creation")
            .clone()
    }
}

impl Factory {
    pub(crate) fn new(core: &Core) -> Self {
        let this = Self {
            inner: new_refcounted(InnerFactory::new()),
        };

        let id = core.next_proxy_id();
        this.inner
            .proxy
            .write()
            .unwrap()
            .replace(Proxy::new(id, &this));
        core.add_proxy(&this, id);

        this
    }

    /// Register for notifications of factory events.
    pub fn add_listener(&self, events: FactoryEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    pub(crate) fn events(&self) -> Arc<Mutex<spa::hook::HookList<FactoryEvents>>> {
        self.inner.hooks.clone()
    }
}

impl InnerFactory {
    fn new() -> Self {
        Self {
            proxy: RwLock::new(None),
            hooks: spa::hook::HookList::new(),
        }
    }
}
