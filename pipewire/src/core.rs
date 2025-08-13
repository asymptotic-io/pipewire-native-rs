// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    os::fd::RawFd,
    rc::Rc,
    sync::{Arc, Mutex},
};

use bitflags::bitflags;
use pipewire_native_spa as spa;

use crate::{
    context::{Context, WeakContext},
    debug, default_topic, hasproxy_method_call, hasproxy_notify,
    id_map::IdMap,
    keys, log,
    properties::Properties,
    protocol,
    proxy::{self, HasProxy, Proxy, ProxyEvents},
    proxy_object_invoke, refcounted, some_closure, types, Id, Refcounted,
};

default_topic!(log::topic::CORE);

pub const VERSION: u32 = 3;

pub const DEFAULT_REMOTE: &str = "pipewire-0";

pub(crate) fn get_remote(props: Option<&spa::dict::Dict>) -> String {
    std::env::var("PIPEWIRE_REMOTE")
        .ok()
        .filter(|v| v.len() > 0)
        .or_else(|| {
            props
                .and_then(|p| p.lookup(keys::REMOTE_NAME).to_owned())
                .filter(|v| v.len() > 0)
                .map(|s| s.to_owned())
        })
        .unwrap_or(DEFAULT_REMOTE.to_owned())
}

refcounted! {
    pub struct Core {
        context: WeakContext,
        properties: Properties,
        client: protocol::client::Client,
        proxy: RefCell<Option<Proxy<Core>>>,
        objects: RefCell<IdMap<Box<dyn HasProxy>>>,
        methods: Rc<RefCell<CoreMethods<Core>>>,
        hooks: Arc<Mutex<spa::hook::HookList<CoreEvents>>>,
    }
}

impl Core {
    pub(crate) fn new(context: &Context, properties: Properties) -> std::io::Result<Self> {
        debug!("Creating new core");

        let this = Self {
            inner: Rc::new(InnerCore::new(context, properties)),
        };

        // Reserve id 0 because we are id 0
        let id = this.inner.objects.borrow_mut().reserve();
        let core_proxy = Proxy::new(0, &this);
        this.inner.proxy.borrow_mut().replace(core_proxy.clone());
        this.inner
            .objects
            .borrow_mut()
            .insert_at(id, Box::new(this.clone()));

        let client = proxy::client::Client::new(&this);
        let client_proxy = client.proxy();

        this.inner.client.set_core(this.downgrade());

        core_proxy.add_listener(ProxyEvents {
            destroy: some_closure!(this, {
                todo!("clean up proxies etc., or delegate to Drop");
            }),
            bound: None,
            removed: None,
            done: None,
            error: None,
            bound_props: None,
        });

        this.add_listener(CoreEvents {
            info: some_closure!(this, info, {
                if let Some(props) = info.props {
                    debug!("updating props {:?}", props);
                    this.context()
                        .update_properties(&props, vec!["default.clock.quantum-limit"]);
                }
            }),
            done: some_closure!(core_proxy, id, seq, {
                debug!("got done: {id} {seq}");
                let core = core_proxy.object().unwrap();
                let proxies = core.inner.objects.borrow();

                if let Some(object) = proxies.get(id) {
                    hasproxy_notify!(object, done, seq);
                }
            }),
            error: some_closure!(core_proxy, id, seq, res, message, {
                debug!("got error: {id} {seq} {res} {message}");
                let core = core_proxy.object().unwrap();
                let proxies = core.inner.objects.borrow();

                if let Some(object) = proxies.get(id) {
                    hasproxy_notify!(object, error, seq, res, message);
                }
            }),
            ping: some_closure!(core_proxy, id, seq, {
                debug!("got ping: {id} {seq}");
                let _ = proxy_object_invoke!(core_proxy, pong, id, seq);
            }),
            remove_id: some_closure!(core_proxy, id, {
                debug!("got remove_id: {id}");
                let core = core_proxy.object().unwrap();
                let proxies = core.inner.objects.borrow();

                if let Some(object) = proxies.get(id) {
                    hasproxy_notify!(object, removed);
                    core.inner.objects.borrow_mut().remove(id);
                }
            }),
            bound_id: some_closure!(core_proxy, id, global_id, {
                debug!("got bound_id: {id} {global_id}");
                let core = core_proxy.object().unwrap();
                let proxies = core.inner.objects.borrow();

                if let Some(object) = proxies.get(id) {
                    hasproxy_method_call!(object, set_bound_id, global_id);
                }
            }),
            add_mem: some_closure!(_core_proxy <- core_proxy, _id, _type_, _fd, _flags, {
                todo!("core.add_mem is not yet implemented")
            }),
            remove_mem: some_closure!(_core_proxy <- core_proxy, _id, {
                todo!("core.remove_mem is not yet implemented")
            }),
            bound_props: some_closure!(core_proxy, id, global_id, props, {
                debug!("got bound_props: {id} {global_id} {props:?}");
                let core = core_proxy.object().unwrap();
                let proxies = core.inner.objects.borrow();

                if let Some(object) = proxies.get(id) {
                    hasproxy_method_call!(object, set_bound_props, global_id, props);
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

    pub(crate) fn context(&self) -> Context {
        self.inner
            .context
            .upgrade()
            .expect("Context should outlive core")
    }

    pub(crate) fn connection(&self) -> protocol::connection::Connection {
        self.inner.client.connection()
    }

    pub(crate) fn next_proxy_id(&self) -> Id {
        self.inner.objects.borrow_mut().reserve()
    }

    pub(crate) fn add_proxy<T: HasProxy + Refcounted>(&self, object: &T, id: Id) {
        self.inner
            .objects
            .borrow_mut()
            .insert_at(id, Box::new(object.clone()));
    }

    pub(crate) fn find_proxy_type(&self, id: Id) -> Option<types::ObjectType> {
        self.inner.objects.borrow().get(id).map(|o| o.type_())
    }

    pub(crate) fn find_proxy<T: HasProxy + Refcounted>(&self, id: Id) -> Option<Proxy<T>> {
        self.inner
            .objects
            .borrow()
            .get(id)
            .and_then(|o| o.downcast_proxy::<T>())
    }

    pub fn add_listener(&self, events: CoreEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    pub fn sync(&self) -> std::io::Result<()> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, sync, 0)
    }

    pub fn registry(&self) -> std::io::Result<proxy::registry::Registry> {
        let proxy = self.proxy();
        proxy_object_invoke!(proxy, get_registry)
    }

    pub(crate) fn methods(&self) -> Rc<RefCell<CoreMethods<Core>>> {
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
            .borrow()
            .as_ref()
            .expect("Proxy should be initialised")
            .clone()
    }
}

bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CoreChangeMask : u32 {
        const PROPS = (1 << 0);
    }
}

pub struct CoreInfo<'a> {
    pub id: u32,
    pub cookie: u32,
    pub user_name: &'a str,
    pub host_name: &'a str,
    pub version: &'a str,
    pub name: &'a str,
    pub mask: CoreChangeMask,
    pub props: Option<&'a Properties>,
}

pub(crate) struct CoreMethods<T: HasProxy + Refcounted> {
    pub(crate) hello: Box<dyn FnMut(&Proxy<T>, u32) -> std::io::Result<()>>,
    pub(crate) sync: Box<dyn FnMut(&Proxy<T>, Id) -> std::io::Result<()>>,
    pub(crate) pong: Box<dyn FnMut(&Proxy<T>, Id, u32) -> std::io::Result<()>>,
    pub(crate) error: Box<dyn FnMut(&Proxy<T>, u32, u32, &str) -> std::io::Result<()>>,
    pub(crate) get_registry:
        Box<dyn FnMut(&Proxy<T>) -> std::io::Result<proxy::registry::Registry>>,
    pub(crate) create_object:
        Box<dyn FnMut(&Proxy<T>, &str, &str, u32, &Properties) -> std::io::Result<()>>,
    pub(crate) destroy: Box<dyn FnMut(&Proxy<T>, Box<dyn HasProxy>) -> std::io::Result<()>>,
}

pub struct CoreEvents {
    pub info: Option<Box<dyn FnMut(&CoreInfo<'_>)>>,
    pub done: Option<Box<dyn FnMut(Id, u32)>>,
    pub error: Option<Box<dyn FnMut(Id, u32, u32, &str)>>,
    pub(crate) ping: Option<Box<dyn FnMut(Id, u32)>>,
    pub(crate) remove_id: Option<Box<dyn FnMut(Id)>>,
    pub(crate) bound_id: Option<Box<dyn FnMut(Id, Id)>>,
    pub(crate) add_mem: Option<Box<dyn FnMut(Id, u32, RawFd, u32)>>,
    pub(crate) remove_mem: Option<Box<dyn FnMut(Id)>>,
    pub(crate) bound_props: Option<Box<dyn FnMut(Id, Id, &Properties)>>,
}

impl InnerCore {
    fn new(context: &Context, mut properties: Properties) -> Self {
        properties.add_dict(&context.properties());

        // TODO: Create mempool

        let client = context.protocol().new_client(None);
        let connection = client.connection();

        Self {
            context: context.downgrade(),
            properties,
            client,
            proxy: RefCell::new(None),
            objects: RefCell::new(IdMap::new()),
            methods: Rc::new(RefCell::new(protocol::marshal::core::Methods::marshal(
                connection,
            ))),
            hooks: spa::hook::HookList::new(),
        }
    }
}
