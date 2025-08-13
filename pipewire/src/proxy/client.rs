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
    permission,
    properties::Properties,
    protocol,
    proxy::{HasProxy, Proxy},
    proxy_object_invoke, refcounted, types, Id, Refcounted,
};

refcounted! {
    pub struct Client {
        proxy: RefCell<Option<Proxy<Client>>>,
        methods: Rc<RefCell<ClientMethods<Client>>>,
        hooks: Arc<Mutex<spa::hook::HookList<ClientEvents>>>,
    }
}

pub(crate) struct ClientMethods<T: HasProxy + Refcounted> {
    pub(crate) error: Box<dyn FnMut(&Proxy<T>, u32, u32, &str) -> std::io::Result<()>>,
    pub(crate) update_properties: Box<dyn FnMut(&Proxy<T>, &Properties) -> std::io::Result<()>>,
    pub(crate) get_permissions: Box<dyn FnMut(&Proxy<T>, u32, u32) -> std::io::Result<()>>,
    pub(crate) update_permissions:
        Box<dyn FnMut(&Proxy<T>, &[permission::Permission]) -> std::io::Result<()>>,
}

bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ClientChangeMask : u32 {
        const PROPS = (1 << 0);
    }
}

pub struct ClientInfo<'a> {
    pub id: Id,
    pub mask: ClientChangeMask,
    pub props: &'a Properties,
}

#[derive(Default)]
pub struct ClientEvents {
    pub info: Option<Box<dyn FnMut(&ClientInfo<'_>)>>,
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
    pub fn new(core: &Core) -> Self {
        let this = Self {
            inner: Rc::new(InnerClient::new(core)),
        };

        let id = core.next_proxy_id();
        this.inner.proxy.borrow_mut().replace(Proxy::new(id, &this));
        core.add_proxy(&this, id);

        this
    }

    pub fn add_listener(&self, events: ClientEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    pub fn error(&self, id: u32, res: u32, message: &str) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, error, id, res, message)
    }

    pub fn get_permissions(&self, index: u32, num: u32) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, get_permissions, index, num)
    }

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
