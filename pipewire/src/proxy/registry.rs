// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use pipewire_native_spa as spa;

use crate::{
    core::Core,
    new_refcounted, permission,
    properties::Properties,
    protocol,
    proxy::{HasProxy, Proxy},
    proxy_object_invoke, refcounted, types, Id, Refcounted,
};

refcounted! {
    /// The registry object allows clients to enumerate and interact with objects. For more
    /// details, see the [Proxy](super::Proxy) documentation.
    pub struct Registry {
        core: Core,
        proxy: RefCell<Option<Proxy<Registry>>>,
        methods: Rc<RefCell<RegistryMethods<Registry>>>,
        hooks: Arc<Mutex<spa::hook::HookList<RegistryEvents>>>,
    }
}

pub(crate) struct RegistryMethods<T: HasProxy + Refcounted> {
    pub bind: Box<dyn FnMut(&Proxy<T>, Id, &str, u32) -> std::io::Result<Box<dyn HasProxy>>>,
    #[allow(unused)]
    pub destroy: Box<dyn FnMut(&Proxy<T>, Id) -> std::io::Result<()>>,
}

/// Events that might be emitted by a [Registry].
#[derive(Default)]
pub struct RegistryEvents {
    /// A global object was exported by the server. The object may be tracked using
    /// [Registry::bind()].
    pub global: Option<Box<dyn FnMut(Id, permission::PermissionBits, &str, u32, &Properties)>>,
    /// A global was removed by the server.
    pub global_remove: Option<Box<dyn FnMut(Id)>>,
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
            .borrow()
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
        this.inner.proxy.borrow_mut().replace(Proxy::new(id, &this));
        core.add_proxy(&this, id);

        this
    }

    pub(crate) fn core(&self) -> Core {
        self.inner.core.clone()
    }

    /// Register to be notified of events on the registry.
    pub fn add_listener(&self, events: RegistryEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    /// "Bind" to a given object, creating a proxy for it that can be used for method calls and
    /// event notifications.
    pub fn bind(&self, id: Id, type_: &str, version: u32) -> std::io::Result<Box<dyn HasProxy>> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, bind, id, type_, version)
    }

    pub(crate) fn methods(&self) -> Rc<RefCell<RegistryMethods<Registry>>> {
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
            proxy: RefCell::new(None),
            methods: Rc::new(RefCell::new(protocol::marshal::registry::Methods::marshal(
                core.connection(),
            ))),
            hooks: spa::hook::HookList::new(),
        }
    }
}
