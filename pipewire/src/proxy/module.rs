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
    /// Proxy that represents a module that is loaded on the server.
    pub struct Module {
        proxy: RwLock<Option<Proxy<Module>>>,
        hooks: Arc<Mutex<spa::hook::HookList<ModuleEvents>>>,
    }
}

bitflags! {
    /// A bit mask of changes signalled in the [ModuleEvents::info] event.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ModuleChangeMask : u32 {
        /// Module properties changed.
        const PROPS = (1 << 0);
    }
}

/// Module information that is provided in a [ModuleEvents::info] event.
pub struct ModuleInfo<'a> {
    /// The ID of the module.
    pub id: Id,
    /// The name of the module
    pub name: &'a str,
    /// The file name the module was loaded from.
    pub file_name: &'a str,
    /// Arguments the module was loaded with.
    pub args: Option<&'a str>,
    /// What changed since the last call.
    pub mask: ModuleChangeMask,
    /// The module's properties.
    pub props: &'a Properties,
}

/// Module events that can be subscribed to.
#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct ModuleEvents {
    /// Module information became available, or changed.
    pub info: Option<Box<dyn FnMut(&ModuleInfo<'_>) + Send>>,
}

impl HasProxy for Module {
    fn type_(&self) -> types::ObjectType {
        types::interface::MODULE
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
            .expect("Module proxy should be initialised on creation")
            .clone()
    }
}

impl Module {
    pub(crate) fn new(core: &Core) -> Self {
        let this = Self {
            inner: new_refcounted(InnerModule::new()),
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

    /// Register for notifications of module events.
    pub fn add_listener(&self, events: ModuleEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    pub(crate) fn events(&self) -> Arc<Mutex<spa::hook::HookList<ModuleEvents>>> {
        self.inner.hooks.clone()
    }
}

impl InnerModule {
    fn new() -> Self {
        Self {
            proxy: RwLock::new(None),
            hooks: spa::hook::HookList::new(),
        }
    }
}
