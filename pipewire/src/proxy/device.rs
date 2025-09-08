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
    protocol,
    proxy::{HasProxy, Proxy},
    proxy_object_invoke, refcounted,
    types::{self, params::ParamBuilder},
    Id, Refcounted,
};

refcounted! {
    /// Proxy that represents a device that is connected to the server.
    pub struct Device {
        proxy: RwLock<Option<Proxy<Device>>>,
        methods: Arc<Mutex<DeviceMethods<Device>>>,
        hooks: Arc<Mutex<spa::hook::HookList<DeviceEvents>>>,
    }
}

#[allow(clippy::type_complexity)]
pub(crate) struct DeviceMethods<T: HasProxy + Refcounted> {
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
    pub(crate) set_param: Box<
        dyn FnMut(
            &Proxy<T>,
            spa::param::ParamType,
            spa::pod::types::ObjectType,
            u32,
            Box<dyn FnOnce(spa::pod::builder::ObjectBuilder) -> spa::pod::builder::ObjectBuilder>,
        ) -> std::io::Result<()>,
    >,
}

bitflags! {
    /// A bit mask of changes signalled in the [DeviceEvents::info] event.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct DeviceChangeMask : u32 {
        /// Device properties changed.
        const PROPS = (1 << 0);
        /// Device params changed.
        const PARAMS = (1 << 1);
    }
}

/// Device information that is provided in a [DeviceEvents::info] event.
pub struct DeviceInfo<'a> {
    /// The ID of the device.
    pub id: Id,
    /// What changed since the last call.
    pub mask: DeviceChangeMask,
    /// The device's properties.
    pub props: &'a Properties,
    /// Device parameters that changed.
    pub params: &'a [(spa::param::ParamType, spa::param::ParamInfoFlags)],
}

/// Device events that can be subscribed to.
#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct DeviceEvents {
    /// Device information became available, or changed.
    pub info: Option<Box<dyn FnMut(&DeviceInfo<'_>) + Send>>,
    /// Device permissions, notified due to a [Device::subscribe_params] or [Device::enum_params]
    /// call.
    pub param:
        Option<Box<dyn FnMut(u32, spa::param::ParamType, u32, u32, &spa::pod::RawPodOwned) + Send>>,
}

impl HasProxy for Device {
    fn type_(&self) -> types::ObjectType {
        types::interface::DEVICE
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
            .expect("Device proxy should be initialised on creation")
            .clone()
    }
}

impl Device {
    pub(crate) fn new(core: &Core) -> Self {
        let this = Self {
            inner: new_refcounted(InnerDevice::new(core)),
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

    /// Register for notifications of device events.
    pub fn add_listener(&self, events: DeviceEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    /// Register for notifications of the specified param types.
    pub fn subscribe_params(&self, ids: &[spa::param::ParamType]) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, subscribe_params, ids)
    }

    /// Enumerate params (via [DeviceEvents::param]). Set `id` to [None] to query all param types.
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

    /// Set a parameter on the device.
    pub fn set_param(
        &self,
        param_id: spa::param::ParamType,
        object_type: spa::pod::types::ObjectType,
        flags: u32,
        builder: Box<
            dyn FnOnce(spa::pod::builder::ObjectBuilder) -> spa::pod::builder::ObjectBuilder,
        >,
    ) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, set_param, param_id, object_type, flags, builder)
    }

    pub(crate) fn methods(&self) -> Arc<Mutex<DeviceMethods<Device>>> {
        self.inner.methods.clone()
    }

    pub(crate) fn events(&self) -> Arc<Mutex<spa::hook::HookList<DeviceEvents>>> {
        self.inner.hooks.clone()
    }
}

impl InnerDevice {
    fn new(core: &Core) -> Self {
        Self {
            proxy: RwLock::new(None),
            methods: Arc::new(Mutex::new(protocol::marshal::device::Methods::marshal(
                core.connection(),
            ))),
            hooks: spa::hook::HookList::new(),
        }
    }
}
