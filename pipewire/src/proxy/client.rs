// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};

use bitflags::bitflags;
use pipewire_native_spa as spa;

use crate::{
    permission,
    properties::Properties,
    proxy::{HasProxy, Proxy},
    refcounted, types, Id, Refcounted,
};

refcounted! {
    pub struct Client {
        proxy: RefCell<Option<Proxy<Client>>>,
        hooks: Arc<Mutex<spa::hook::HookList<ClientEvents>>>,
    }
}

impl HasProxy for Client {
    fn type_(&self) -> types::ObjectType {
        types::interface::CLIENT
    }

    fn version() -> u32 {
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

pub struct ClientEvents {
    pub info: Box<dyn FnMut(ClientInfo<'_>)>,
    pub permissions: Box<dyn FnMut(u32, u32, permission::Permission)>,
}

impl Client {
    pub fn new(id: Id) -> Self {
        let this = Self {
            inner: Rc::new(InnerClient::new()),
        };

        this.inner
            .proxy
            .borrow_mut()
            .replace(Proxy::new_weak(id, &this));

        this
    }
}

impl InnerClient {
    fn new() -> Self {
        Self {
            proxy: RefCell::new(None),
            hooks: spa::hook::HookList::new(),
        }
    }
}
