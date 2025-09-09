// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::sync::{Arc, Mutex, RwLock};

use pipewire_native_spa as spa;

use crate::{
    core::Core,
    new_refcounted, permission,
    properties::Properties,
    protocol,
    proxy::{HasProxy, Proxy},
    proxy_object_invoke, refcounted, types, HookId, Id, Refcounted,
};

refcounted! {
    /// The registry object allows clients to enumerate and interact with objects. For more
    /// details, see the [Proxy](super::Proxy) documentation.
    pub struct Registry {
        core: Core,
        proxy: RwLock<Option<Proxy<Registry>>>,
        methods: Arc<Mutex<RegistryMethods<Registry>>>,
        hooks: Arc<Mutex<spa::hook::HookList<RegistryEvents>>>,
    }
}

#[allow(clippy::type_complexity)]
pub(crate) struct RegistryMethods<T: HasProxy + Refcounted> {
    pub bind: Box<dyn FnMut(&Proxy<T>, Id, &str, u32) -> std::io::Result<Box<dyn HasProxy>>>,
    pub destroy: Box<dyn FnMut(&Proxy<T>, Id) -> std::io::Result<()>>,
}

/// Events that might be emitted by a [Registry].
#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct RegistryEvents {
    /// A global object was exported by the server. The object may be tracked using
    /// [Registry::bind()].
    pub global:
        Option<Box<dyn FnMut(Id, permission::PermissionBits, &str, u32, &Properties) + Send>>,
    /// A global was removed by the server.
    pub global_remove: Option<Box<dyn FnMut(Id) + Send>>,
}

impl HasProxy for Registry {
    fn type_(&self) -> types::ObjectType {
        types::interface::REGISTRY
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
            .expect("Registry proxy should be initialised on creation")
            .clone()
    }
}

impl Registry {
    pub(crate) fn new(core: &Core) -> Self {
        let this = Self {
            inner: new_refcounted(InnerRegistry::new(core)),
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

    pub(crate) fn core(&self) -> Core {
        self.inner.core.clone()
    }

    /// Register to be notified of events on the registry.
    pub fn add_listener(&self, events: RegistryEvents) -> HookId {
        self.inner.hooks.lock().unwrap().append(events)
    }

    /// Remove a set of event listeners.
    pub fn remove_listener(&self, hook_id: HookId) {
        self.inner.hooks.lock().unwrap().remove(hook_id);
    }

    /// "Bind" to a given object, creating a proxy for it that can be used for method calls and
    /// event notifications.
    pub fn bind(&self, id: Id, type_: &str, version: u32) -> std::io::Result<Box<dyn HasProxy>> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, bind, id, type_, version)
    }

    /// Try to destroy the global object corresponding to this proxy. This may fail if the client
    /// does not have sufficient permissions.
    pub fn destroy(&self, id: Id) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, destroy, id)
    }

    pub(crate) fn methods(&self) -> Arc<Mutex<RegistryMethods<Registry>>> {
        self.inner.methods.clone()
    }

    pub(crate) fn events(&self) -> Arc<Mutex<spa::hook::HookList<RegistryEvents>>> {
        self.inner.hooks.clone()
    }
}

impl InnerRegistry {
    fn new(core: &Core) -> Self {
        Self {
            core: core.clone(),
            proxy: RwLock::new(None),
            methods: Arc::new(Mutex::new(protocol::marshal::registry::Methods::marshal(
                core.connection(),
            ))),
            hooks: spa::hook::HookList::new(),
        }
    }
}
