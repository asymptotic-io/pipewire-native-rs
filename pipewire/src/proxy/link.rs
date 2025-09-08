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
    proxy::{HasProxy, Proxy},
    refcounted, types, Id,
};

refcounted! {
    /// Proxy that represents a link that is loaded on the server.
    pub struct Link {
        proxy: RwLock<Option<Proxy<Link>>>,
        hooks: Arc<Mutex<spa::hook::HookList<LinkEvents>>>,
    }
}

bitflags! {
    /// A bit mask of changes signalled in the [LinkEvents::info] event.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LinkChangeMask : u32 {
        /// Link properties changed.
        const PROPS = (1 << 0);
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, macros::EnumU32)]
/// Represents the current state of a link.
pub enum LinkState {
    /// Link is in an error state.
    Error,
    /// Link is unlinked.
    Unlinked,
    /// Link is initialised.
    Init,
    /// Link is negotiation formats.
    Negotiation,
    /// Link is allocating buffers.
    Allocating,
    /// Link is paused.
    Paused,
    /// Link is active.
    Active,
}

/// Link information that is provided in a [LinkEvents::info] event.
pub struct LinkInfo<'a> {
    /// The ID of the link.
    pub id: Id,
    /// Global ID of the output node for this link.
    pub output_node_id: Id,
    /// Global ID of the output port for this link.
    pub output_port_id: Id,
    /// Global ID of the input node for this link.
    pub input_node_id: Id,
    /// Global ID of the input port for this link.
    pub input_port_id: Id,
    /// What changed since the last call.
    pub mask: LinkChangeMask,
    /// Current state of the link.
    pub state: LinkState,
    /// Error if `state` is [LinkState::Error].
    pub error: Option<&'a str>,
    /// Format of the link, if negotiated.
    pub format: Option<&'a spa::pod::RawPodOwned>,
    /// The link's properties.
    pub props: &'a Properties,
}

/// Link events that can be subscribed to.
#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct LinkEvents {
    /// Link information became available, or changed.
    pub info: Option<Box<dyn FnMut(&LinkInfo<'_>) + Send>>,
}

impl HasProxy for Link {
    fn type_(&self) -> types::ObjectType {
        types::interface::LINK
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
            .expect("Link proxy should be initialised on creation")
            .clone()
    }
}

impl Link {
    pub(crate) fn new(core: &Core) -> Self {
        let this = Self {
            inner: new_refcounted(InnerLink::new()),
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

    /// Register for notifications of link events.
    pub fn add_listener(&self, events: LinkEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    pub(crate) fn events(&self) -> Arc<Mutex<spa::hook::HookList<LinkEvents>>> {
        self.inner.hooks.clone()
    }
}

impl InnerLink {
    fn new() -> Self {
        Self {
            proxy: RwLock::new(None),
            hooks: spa::hook::HookList::new(),
        }
    }
}
