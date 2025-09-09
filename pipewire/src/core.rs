// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    os::fd::RawFd,
    sync::{Arc, Mutex, RwLock},
};

use bitflags::bitflags;
use pipewire_native_spa as spa;

use crate::{
    context::{Context, WeakContext},
    debug, default_topic, hasproxy_method_call_unlocked, hasproxy_notify, hasproxy_notify_unlocked,
    id_map::IdMap,
    keys, log, new_refcounted,
    properties::Properties,
    protocol,
    proxy::{self, HasProxy, Proxy, ProxyEvents},
    proxy_notify, proxy_object_invoke, refcounted, some_closure, types, HookId, Id, Refcounted,
};

default_topic!(log::topic::CORE);

const VERSION: u32 = 4;

const DEFAULT_REMOTE: &str = "pipewire-0";

pub(crate) fn get_remote(props: Option<&spa::dict::Dict>) -> String {
    std::env::var("PIPEWIRE_REMOTE")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| {
            props
                .and_then(|p| p.lookup(keys::REMOTE_NAME).to_owned())
                .filter(|v| !v.is_empty())
                .map(|s| s.to_owned())
        })
        .unwrap_or(DEFAULT_REMOTE.to_owned())
}

refcounted! {
    /// A singleton object representing the connection between the client and the PipeWire server.
    pub struct Core {
        context: WeakContext,
        properties: Properties,
        client: protocol::client::Client,
        destroyed: RwLock<bool>,
        proxy: RwLock<Option<Proxy<Core>>>,
        objects: RwLock<IdMap<Box<dyn HasProxy>>>,
        methods: Arc<Mutex<CoreMethods<Core>>>,
        hooks: Arc<Mutex<spa::hook::HookList<CoreEvents>>>,
    }
}

impl Core {
    pub(crate) fn new(context: &Context, properties: Properties) -> std::io::Result<Self> {
        debug!("Creating new core");

        let this = Self {
            inner: new_refcounted(InnerCore::new(context, properties)),
        };

        // Reserve id 0 because we are id 0
        let id = this.inner.objects.write().unwrap().reserve();
        let core_proxy = Proxy::new(0, &this);
        this.inner
            .proxy
            .write()
            .unwrap()
            .replace(core_proxy.clone());
        this.inner
            .objects
            .write()
            .unwrap()
            .insert_at(id, Box::new(this.clone()));

        let client = proxy::client::Client::new(&this);
        let client_proxy = client.proxy();

        this.inner.client.set_core(this.downgrade());

        core_proxy.add_listener(ProxyEvents {
            destroy: some_closure!([this] {
                debug!("core destroy");
                let mut destroyed = this.inner.destroyed.write().unwrap();

                if *destroyed {
                    return;
                }

                *destroyed = true;

                let mut objects = this.inner.objects.write().unwrap();
                let client = objects.get(1).unwrap();

                hasproxy_notify!(client, destroy);
                objects.clear();

                this.inner.client.disconnect();
            }),
            removed: some_closure!([this] {
                debug!("core removed");
                for o in this
                    .inner
                    .objects
                    .read()
                    .unwrap()
                    .iter()
                    .skip(1) // first object is core, so skip it
                    .map(|(_id, object)| object)
                {
                    hasproxy_notify!(o, removed)
                }
            }),
            ..Default::default()
        });

        this.add_listener(CoreEvents {
            info: some_closure!([this] info, {
                if let Some(props) = info.props {
                    debug!("updating props {:?}", props);
                    this.context()
                        .update_properties(props, vec!["default.clock.quantum-limit"]);
                }
            }),
            done: some_closure!([core_proxy] id, seq, {
                debug!("got done: {id} {seq}");
                let core = core_proxy.object().unwrap();
                let proxies = core.inner.objects.read().unwrap();

                if let Some(object) = proxies.get(id) {
                    hasproxy_notify_unlocked!(object, proxies, done, seq);
                }
            }),
            error: some_closure!([core_proxy] id, seq, res, message, {
                debug!("got error: {id} {seq} {res} {message}");
                let core = core_proxy.object().unwrap();
                let proxies = core.inner.objects.read().unwrap();

                if let Some(object) = proxies.get(id) {
                    hasproxy_notify_unlocked!(object, proxies, error, seq, res, message);
                }
            }),
            ping: some_closure!([core_proxy] id, seq, {
                debug!("got ping: {id} {seq}");
                let _ = proxy_object_invoke!(core_proxy, pong, id, seq);
            }),
            remove_id: some_closure!([core_proxy] id, {
                debug!("got remove_id: {id}");
                let core = core_proxy.object().unwrap();
                let mut proxies = core.inner.objects.write().unwrap();

                if let Some(object) = proxies.get(id) {
                    hasproxy_notify!(object, removed);
                    proxies.remove(id);
                }
            }),
            bound_id: some_closure!([core_proxy] id, global_id, {
                debug!("got bound_id: {id} {global_id}");
                let core = core_proxy.object().unwrap();
                let proxies = core.inner.objects.read().unwrap();

                if let Some(object) = proxies.get(id) {
                    hasproxy_method_call_unlocked!(object, proxies, set_bound_id, global_id);
                }
            }),
            add_mem: some_closure!([] _id, _type_, _fd, _flags, {
                todo!("core.add_mem is not yet implemented")
            }),
            remove_mem: some_closure!([] _id, {
                todo!("core.remove_mem is not yet implemented")
            }),
            bound_props: some_closure!([core_proxy] id, global_id, props, {
                debug!("got bound_props: {id} {global_id} {props:?}");
                let core = core_proxy.object().unwrap();
                let proxies = core.inner.objects.read().unwrap();

                if let Some(object) = proxies.get(id) {
                    hasproxy_method_call_unlocked!(object, proxies, set_bound_props, global_id, props);
                }
            }),
        });

        proxy_object_invoke!(core_proxy, hello, VERSION)?;

        proxy_object_invoke!(client_proxy, update_properties, &this.inner.properties)?;

        this.inner
            .client
            .connect(Some(&this.inner.properties.dict()), None)?;

        Ok(this)
    }

    /// Disconnect connection with the PipeWire server. This will immediately trigger the `removed`
    /// and `destroy` events on all tracked proxies. Callers should ensure that this will not
    /// result in deadlocks with their own synchronisation primitives (for example, taking a lock
    /// before disconnecting that is also taken in either of those callbacks).
    pub fn disconnect(&self) {
        proxy_notify!(self, removed);
        proxy_notify!(self, destroy);
    }

    pub(crate) fn context(&self) -> Context {
        self.inner
            .context
            .upgrade()
            .expect("Context should outlive core")
    }

    pub(crate) fn connection(&self) -> protocol::connection::Connection {
        self.inner.client.connection()
    }

    pub(crate) fn new_object(&self, type_: &str) -> std::io::Result<Box<dyn HasProxy>> {
        let new_object: Box<dyn HasProxy> = match type_ {
            types::interface::CLIENT => Box::new(proxy::client::Client::new(self)),
            types::interface::DEVICE => Box::new(proxy::device::Device::new(self)),
            types::interface::FACTORY => Box::new(proxy::factory::Factory::new(self)),
            types::interface::LINK => Box::new(proxy::link::Link::new(self)),
            types::interface::METADATA => Box::new(proxy::metadata::Metadata::new(self)),
            types::interface::MODULE => Box::new(proxy::module::Module::new(self)),
            types::interface::NODE => Box::new(proxy::node::Node::new(self)),
            types::interface::PORT => Box::new(proxy::port::Port::new(self)),
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    format!("Unsupported proxy type {type_}"),
                ))
            }
        };

        Ok(new_object)
    }

    pub(crate) fn next_proxy_id(&self) -> Id {
        self.inner.objects.write().unwrap().reserve()
    }

    pub(crate) fn add_proxy<T: HasProxy + Refcounted>(&self, object: &T, id: Id) {
        self.inner
            .objects
            .write()
            .unwrap()
            .insert_at(id, Box::new(object.clone()));
    }

    pub(crate) fn find_proxy_type(&self, id: Id) -> Option<types::ObjectType> {
        self.inner
            .objects
            .read()
            .unwrap()
            .get(id)
            .map(|o| o.type_())
    }

    pub(crate) fn find_proxy<T: HasProxy + Refcounted>(&self, id: Id) -> Option<Proxy<T>> {
        self.inner
            .objects
            .read()
            .unwrap()
            .get(id)
            .and_then(|o| o.downcast_proxy::<T>())
    }

    /// Listen for events on the core object.
    pub fn add_listener(&self, events: CoreEvents) -> HookId {
        self.inner.hooks.lock().unwrap().append(events)
    }

    /// Remove a set of event listeners.
    pub fn remove_listener(&self, hook_id: HookId) {
        self.inner.hooks.lock().unwrap().remove(hook_id);
    }

    /// Trigger a `sync` message to the server, flushing all pending messages.
    pub fn sync(&self) -> std::io::Result<u32> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, sync, 0)
    }

    /// Retrieve a [Registry](proxy::registry::Registry). This can be used to query and track
    /// objects exposed by the server.
    pub fn registry(&self) -> std::io::Result<proxy::registry::Registry> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, get_registry)
    }

    /// Create an object of the given factory type on the server.
    pub fn create_object(
        &self,
        factory_name: &str,
        type_: &str,
        version: u32,
        props: &Properties,
    ) -> std::io::Result<Box<dyn HasProxy>> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, create_object, factory_name, type_, version, props)
    }

    /// Destroy a proxy.
    pub fn destroy(&self, object: &dyn HasProxy) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, destroy, object)
    }

    pub(crate) fn methods(&self) -> Arc<Mutex<CoreMethods<Core>>> {
        self.inner.methods.clone()
    }

    pub(crate) fn events(&self) -> Arc<Mutex<spa::hook::HookList<CoreEvents>>> {
        self.inner.hooks.clone()
    }
}

impl HasProxy for Core {
    fn type_(&self) -> types::ObjectType {
        types::interface::CORE
    }

    fn version(&self) -> u32 {
        4
    }

    fn proxy(&self) -> Proxy<Core> {
        self.inner
            .proxy
            .read()
            .unwrap()
            .as_ref()
            .expect("Proxy should be initialised")
            .clone()
    }
}

bitflags! {
    /// Indicates what changes are being signalled in a [CoreEvents::info] event.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CoreChangeMask : u32 {
        /// Properties changed.
        const PROPS = (1 << 0);
    }
}

/// Provides [Core]-related information in the [CoreEvents::info] event.
pub struct CoreInfo<'a> {
    /// Id of the core.
    pub id: u32,
    /// Random cookie to identify this instance.
    pub cookie: u32,
    /// User name of the user who started the core.
    pub user_name: &'a str,
    /// Host name on which the core is running.
    pub host_name: &'a str,
    /// Interface version of the core.
    pub version: &'a str,
    /// Name of the core.
    pub name: &'a str,
    /// Set of changes since the last call.
    pub mask: CoreChangeMask,
    /// Properties of the core.
    pub props: Option<&'a Properties>,
}

#[allow(clippy::type_complexity)]
pub(crate) struct CoreMethods<T: HasProxy + Refcounted> {
    pub(crate) hello: Box<dyn FnMut(&Proxy<T>, u32) -> std::io::Result<()>>,
    pub(crate) sync: Box<dyn FnMut(&Proxy<T>, Id) -> std::io::Result<u32>>,
    pub(crate) pong: Box<dyn FnMut(&Proxy<T>, Id, u32) -> std::io::Result<()>>,
    #[allow(unused)]
    pub(crate) error: Box<dyn FnMut(&Proxy<T>, u32, u32, &str) -> std::io::Result<()>>,
    pub(crate) get_registry:
        Box<dyn FnMut(&Proxy<T>) -> std::io::Result<proxy::registry::Registry>>,
    pub(crate) create_object: Box<
        dyn FnMut(&Proxy<T>, &str, &str, u32, &Properties) -> std::io::Result<Box<dyn HasProxy>>,
    >,
    pub(crate) destroy: Box<dyn FnMut(&Proxy<T>, &dyn HasProxy) -> std::io::Result<()>>,
}

/// Events that may be emitted by a [Core] proxy object.
#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct CoreEvents {
    /// Information about the core changed.
    pub info: Option<Box<dyn FnMut(&CoreInfo<'_>) + Send>>,
    /// A core operation was completed.
    pub done: Option<Box<dyn FnMut(Id, u32) + Send>>,
    /// An error occurred on the core.
    pub error: Option<Box<dyn FnMut(Id, u32, u32, &str) + Send>>,
    pub(crate) ping: Option<Box<dyn FnMut(Id, u32) + Send>>,
    pub(crate) remove_id: Option<Box<dyn FnMut(Id) + Send>>,
    pub(crate) bound_id: Option<Box<dyn FnMut(Id, Id) + Send>>,
    #[allow(unused)]
    pub(crate) add_mem: Option<Box<dyn FnMut(Id, u32, RawFd, u32) + Send>>,
    #[allow(unused)]
    pub(crate) remove_mem: Option<Box<dyn FnMut(Id) + Send>>,
    pub(crate) bound_props: Option<Box<dyn FnMut(Id, Id, &Properties) + Send>>,
}

impl InnerCore {
    fn new(context: &Context, mut properties: Properties) -> Self {
        properties.add_dict(&context.properties_dict());

        // TODO: Create mempool

        let client = context.protocol().new_client(None);
        let connection = client.connection();

        Self {
            context: context.downgrade(),
            properties,
            client,
            destroyed: RwLock::new(false),
            proxy: RwLock::new(None),
            objects: RwLock::new(IdMap::new()),
            methods: Arc::new(Mutex::new(protocol::marshal::core::Methods::marshal(
                connection,
            ))),
            hooks: spa::hook::HookList::new(),
        }
    }
}
