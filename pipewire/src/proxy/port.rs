// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::sync::{Arc, Mutex, RwLock};

use bitflags::bitflags;
use pipewire_native_macros as macros;
use pipewire_native_spa as spa;

use crate::{
    core::Core,
    new_refcounted,
    properties::Properties,
    protocol,
    proxy::{HasProxy, Proxy},
    proxy_object_invoke, refcounted,
    types::{self, params::ParamBuilder},
    Id, Refcounted,
};

refcounted! {
    /// Proxy that represents a port that is connected to the server.
    pub struct Port {
        proxy: RwLock<Option<Proxy<Port>>>,
        methods: Arc<Mutex<PortMethods<Port>>>,
        hooks: Arc<Mutex<spa::hook::HookList<PortEvents>>>,
    }
}

#[allow(clippy::type_complexity)]
pub(crate) struct PortMethods<T: HasProxy + Refcounted> {
    pub(crate) subscribe_params:
        Box<dyn FnMut(&Proxy<T>, &[spa::param::ParamType]) -> std::io::Result<()>>,
    pub(crate) enum_params: Box<
        dyn FnMut(
            &Proxy<T>,
            u32,
            Option<spa::param::ParamType>,
            u32,
            u32,
            Option<ParamBuilder>,
        ) -> std::io::Result<()>,
    >,
}

bitflags! {
    /// A bit mask of changes signalled in the [PortEvents::info] event.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PortChangeMask : u32 {
        /// Port properties changed.
        const PROPS = (1 << 0);
        /// Port params changed.
        const PARAMS = (1 << 1);
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, macros::EnumU32)]
/// Represents the direction of data flow for the port.
pub enum PortDirection {
    /// Input port.
    Input,
    /// Output port.
    Output,
}

/// Port information that is provided in a [PortEvents::info] event.
pub struct PortInfo<'a> {
    /// The ID of the port.
    pub id: Id,
    /// The direction of the port
    pub direction: PortDirection,
    /// What changed since the last call.
    pub mask: PortChangeMask,
    /// The port's properties.
    pub props: &'a Properties,
    /// Port parameters that changed.
    pub params: &'a [(spa::param::ParamType, spa::param::ParamInfoFlags)],
}

/// Port events that can be subscribed to.
#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct PortEvents {
    /// Port information became available, or changed.
    pub info: Option<Box<dyn FnMut(&PortInfo<'_>) + Send>>,
    /// Port permissions, notified due to a [Port::subscribe_params] or [Port::enum_params]
    /// call.
    pub param:
        Option<Box<dyn FnMut(u32, spa::param::ParamType, u32, u32, &spa::pod::RawPodOwned) + Send>>,
}

impl HasProxy for Port {
    fn type_(&self) -> types::ObjectType {
        types::interface::PORT
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
            .expect("Port proxy should be initialised on creation")
            .clone()
    }
}

impl Port {
    pub(crate) fn new(core: &Core) -> Self {
        let this = Self {
            inner: new_refcounted(InnerPort::new(core)),
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

    /// Register for notifications of port events.
    pub fn add_listener(&self, events: PortEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    /// Register for notifications of the specified param types.
    pub fn subscribe_params(&self, ids: &[spa::param::ParamType]) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, subscribe_params, ids)
    }

    /// Enumerate params (via [PortEvents::param]). Set `id` to [None] to query all param types.
    pub fn enum_params(
        &self,
        seq: u32,
        id: Option<spa::param::ParamType>,
        start: u32,
        num: u32,
        filter: Option<ParamBuilder>,
    ) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, enum_params, seq, id, start, num, filter)
    }

    pub(crate) fn methods(&self) -> Arc<Mutex<PortMethods<Port>>> {
        self.inner.methods.clone()
    }

    pub(crate) fn events(&self) -> Arc<Mutex<spa::hook::HookList<PortEvents>>> {
        self.inner.hooks.clone()
    }
}

impl InnerPort {
    fn new(core: &Core) -> Self {
        Self {
            proxy: RwLock::new(None),
            methods: Arc::new(Mutex::new(protocol::marshal::port::Methods::marshal(
                core.connection(),
            ))),
            hooks: spa::hook::HookList::new(),
        }
    }
}
