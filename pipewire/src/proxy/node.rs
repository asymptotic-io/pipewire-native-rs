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
    /// Proxy that represents a node that is connected to the server.
    pub struct Node {
        proxy: RwLock<Option<Proxy<Node>>>,
        methods: Arc<Mutex<NodeMethods<Node>>>,
        hooks: Arc<Mutex<spa::hook::HookList<NodeEvents>>>,
    }
}

#[allow(clippy::type_complexity)]
pub(crate) struct NodeMethods<T: HasProxy + Refcounted> {
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
    pub(crate) send_command: Box<
        dyn FnMut(
            &Proxy<T>,
            Box<dyn FnOnce(spa::pod::builder::Builder) -> spa::pod::builder::Builder>,
        ) -> std::io::Result<()>,
    >,
}

bitflags! {
    /// A bit mask of changes signalled in the [NodeEvents::info] event.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NodeChangeMask : u32 {
        /// Input ports changed.
        const INPUT_PORTS = (1 << 0);
        /// Output ports changed.
        const OUTPUT_PORTS = (1 << 1);
        /// Node state changed.
        const STATE = (1 << 2);
        /// Node properties changed.
        const PROPS = (1 << 3);
        /// Node params changed.
        const PARAMS = (1 << 4);
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, macros::EnumU32)]
/// Represents the current state of a node.
pub enum NodeState {
    /// Node is in an error state.
    Error,
    /// Node is being created.
    Creating,
    /// Node is suspended.
    Suspended,
    /// Node is running, but no port is active.
    Idle,
    /// Node is running.
    Running,
}

/// Node information that is provided in a [NodeEvents::info] event.
pub struct NodeInfo<'a> {
    /// The ID of the node.
    pub id: Id,
    /// Maximum number of input ports.
    pub max_input_ports: u32,
    /// Maximum number of output ports.
    pub max_output_ports: u32,
    /// What changed since the last call.
    pub mask: NodeChangeMask,
    /// Number of input ports.
    pub n_input_ports: u32,
    /// Number of output ports.
    pub n_output_ports: u32,
    /// The node's current state.
    pub state: NodeState,
    /// The error reason if `state` is [NodeState::Error].
    pub error: Option<&'a str>,
    /// The node's properties.
    pub props: &'a Properties,
    /// Node parameters that changed.
    pub params: &'a [(spa::param::ParamType, spa::param::ParamInfoFlags)],
}

/// Node events that can be subscribed to.
#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct NodeEvents {
    /// Node information became available, or changed.
    pub info: Option<Box<dyn FnMut(&NodeInfo<'_>) + Send>>,
    /// Node permissions, notified due to a [Node::subscribe_params] or [Node::enum_params]
    /// call.
    pub param:
        Option<Box<dyn FnMut(u32, spa::param::ParamType, u32, u32, &spa::pod::RawPodOwned) + Send>>,
}

impl HasProxy for Node {
    fn type_(&self) -> types::ObjectType {
        types::interface::NODE
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
            .expect("Node proxy should be initialised on creation")
            .clone()
    }
}

impl Node {
    pub(crate) fn new(core: &Core) -> Self {
        let this = Self {
            inner: new_refcounted(InnerNode::new(core)),
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

    /// Register for notifications of node events.
    pub fn add_listener(&self, events: NodeEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    /// Register for notifications of the specified param types.
    pub fn subscribe_params(&self, ids: &[spa::param::ParamType]) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, subscribe_params, ids)
    }

    /// Enumerate params (via [NodeEvents::param]). Set `id` to [None] to query all param types.
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

    /// Set a parameter on the node.
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

    /// Send a command to the node.
    pub fn send_command(
        &self,
        builder: Box<dyn FnOnce(spa::pod::builder::Builder) -> spa::pod::builder::Builder>,
    ) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, send_command, builder)
    }

    pub(crate) fn methods(&self) -> Arc<Mutex<NodeMethods<Node>>> {
        self.inner.methods.clone()
    }

    pub(crate) fn events(&self) -> Arc<Mutex<spa::hook::HookList<NodeEvents>>> {
        self.inner.hooks.clone()
    }
}

impl InnerNode {
    fn new(core: &Core) -> Self {
        Self {
            proxy: RwLock::new(None),
            methods: Arc::new(Mutex::new(protocol::marshal::node::Methods::marshal(
                core.connection(),
            ))),
            hooks: spa::hook::HookList::new(),
        }
    }
}
