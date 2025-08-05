// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    os::fd::RawFd,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};

use bitflags::bitflags;
use pipewire_native_spa as spa;

use crate::{
    context::{Context, WeakContext},
    debug, default_topic,
    id_map::IdMap,
    keys, log,
    properties::Properties,
    protocol,
    proxy::{self, HasProxy, Proxy, ProxyEvents},
    proxy_notify_dyn, proxy_object_invoke, refcounted, some_closure, types, Id, Refcounted,
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
        proxies: RefCell<IdMap<Box<dyn HasProxy>>>,
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
        let id = this.inner.proxies.borrow_mut().reserve();
        let core_proxy = Proxy::new_weak(0, &this);
        this.inner.proxy.borrow_mut().replace(core_proxy.clone());
        this.inner
            .proxies
            .borrow_mut()
            .insert_at(id, Box::new(this.clone()));

        let id = this.inner.proxies.borrow_mut().reserve();
        let client = proxy::client::Client::new(id);
        this.inner
            .proxies
            .borrow_mut()
            .insert_at(id, Box::new(client));

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
                        .update_properties(props, vec!["default.clock.quantum-limit"]);
                }
            }),
            done: some_closure!(core_proxy, id, seq, {
                debug!("got done: {id} {seq}");
                let core = core_proxy.object().unwrap();
                let proxies = core.inner.proxies.borrow();

                if let Some(object) = proxies.get(id) {
                    proxy_notify_dyn!(object, done, seq);
                }
            }),
            error: None,
            ping: some_closure!(core_proxy, id, seq, {
                debug!("got ping: {id} {seq}");
                let _ = proxy_object_invoke!(core_proxy, pong, id, seq);
            }),
            remove_id: None,
            bound_id: None,
            add_mem: None,
            remove_mem: None,
            bound_props: None,
        });

        proxy_object_invoke!(core_proxy, hello, VERSION)?;
        // update client properties

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

    pub(crate) fn find_proxy_type(&self, id: Id) -> Option<types::ObjectType> {
        self.inner.proxies.borrow().get(id).map(|o| o.type_())
    }

    pub(crate) fn find_proxy<T: HasProxy + Refcounted>(&self, id: Id) -> Option<Proxy<T>> {
        self.inner
            .proxies
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

    fn version() -> u32 {
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
    pub props: Option<&'a spa::dict::Dict>,
}

pub struct CoreMethods<T: HasProxy + Refcounted> {
    pub(crate) hello: Box<dyn FnMut(&Proxy<T>, u32) -> std::io::Result<()>>,
    pub(crate) sync: Box<dyn FnMut(&Proxy<T>, Id) -> std::io::Result<()>>,
    pub(crate) pong: Box<dyn FnMut(&Proxy<T>, Id, u32) -> std::io::Result<()>>,
    pub(crate) error: Box<dyn FnMut(&Proxy<T>, u32, u32, &str) -> std::io::Result<()>>,
    // pub(crate) get_registry: fn(...)
    pub(crate) create_object: Box<dyn FnMut(&Proxy<T>, &str, &str, u32, &spa::dict::Dict)>,
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
    pub(crate) bound_props: Option<Box<dyn FnMut(Id, Id, &spa::dict::Dict)>>,
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
            proxies: RefCell::new(IdMap::new()),
            methods: Rc::new(RefCell::new(protocol::marshal::core::Methods::marshal(
                connection,
            ))),
            hooks: spa::hook::HookList::new(),
        }
    }
}
