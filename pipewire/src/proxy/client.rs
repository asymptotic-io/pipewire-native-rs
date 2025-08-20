// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use bitflags::bitflags;
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
    /// Proxy that represents a client that is connected to the server.
    pub struct Client {
        proxy: RefCell<Option<Proxy<Client>>>,
        methods: Rc<RefCell<ClientMethods<Client>>>,
        hooks: Arc<Mutex<spa::hook::HookList<ClientEvents>>>,
    }
}

#[allow(clippy::type_complexity)]
pub(crate) struct ClientMethods<T: HasProxy + Refcounted> {
    pub(crate) error: Box<dyn FnMut(&Proxy<T>, u32, u32, &str) -> std::io::Result<()>>,
    pub(crate) update_properties: Box<dyn FnMut(&Proxy<T>, &Properties) -> std::io::Result<()>>,
    pub(crate) get_permissions: Box<dyn FnMut(&Proxy<T>, u32, u32) -> std::io::Result<()>>,
    pub(crate) update_permissions:
        Box<dyn FnMut(&Proxy<T>, &[permission::Permission]) -> std::io::Result<()>>,
}

bitflags! {
    /// A bit mask of changes signalled in the [ClientEvents::info] event.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ClientChangeMask : u32 {
        /// Client properties changed.
        const PROPS = (1 << 0);
    }
}

/// Client information that is provided in a [ClientEvents::info] event.
pub struct ClientInfo<'a> {
    /// The ID of the client.
    pub id: Id,
    /// What changed since the last call.
    pub mask: ClientChangeMask,
    /// The client's properties.
    pub props: &'a Properties,
}

/// Client events that can be subscribed to.
#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct ClientEvents {
    /// Client information became available, or changed.
    pub info: Option<Box<dyn FnMut(&ClientInfo<'_>)>>,
    /// Client permissions, notified due to a [Client::permissions()] call.
    pub permissions: Option<Box<dyn FnMut(u32, &[permission::Permission])>>,
}

impl HasProxy for Client {
    fn type_(&self) -> types::ObjectType {
        types::interface::CLIENT
    }

    fn version(&self) -> u32 {
        3
    }

    fn proxy(&self) -> Proxy<Self> {
        self.inner
            .proxy
            .borrow()
            .as_ref()
            .expect("Client proxy should be initialised on creation")
            .clone()
    }
}

impl Client {
    pub(crate) fn new(core: &Core) -> Self {
        let this = Self {
            inner: new_refcounted(InnerClient::new(core)),
        };

        let id = core.next_proxy_id();
        this.inner.proxy.borrow_mut().replace(Proxy::new(id, &this));
        core.add_proxy(&this, id);

        this
    }

    /// Register for notifications of client events.
    pub fn add_listener(&self, events: ClientEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    /// Signal an error to the client.
    pub fn error(&self, id: u32, res: u32, message: &str) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, error, id, res, message)
    }

    /// Retrieve permissions of a client.
    pub fn permissions(&self, index: u32, num: u32) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, get_permissions, index, num)
    }

    /// Update permissions of a client.
    pub fn update_permissions(
        &self,
        permissions: &[permission::Permission],
    ) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, update_permissions, permissions)
    }

    pub(crate) fn methods(&self) -> Rc<RefCell<ClientMethods<Client>>> {
        self.inner.methods.clone()
    }

    pub(crate) fn events(&self) -> Arc<Mutex<spa::hook::HookList<ClientEvents>>> {
        self.inner.hooks.clone()
    }
}

impl InnerClient {
    fn new(core: &Core) -> Self {
        Self {
            proxy: RefCell::new(None),
            methods: Rc::new(RefCell::new(protocol::marshal::client::Methods::marshal(
                core.connection(),
            ))),
            hooks: spa::hook::HookList::new(),
        }
    }
}
