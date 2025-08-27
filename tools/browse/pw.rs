// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

use pipewire::{
    self,
    context::Context,
    core::Core,
    keys,
    properties::Properties,
    proxy::{
        self,
        client::{ClientEvents, ClientInfo},
        device::{DeviceEvents, DeviceInfo},
        module::{ModuleEvents, ModuleInfo},
        node::{NodeEvents, NodeInfo},
        registry::{Registry, RegistryEvents},
        HasProxy, ProxyEvents,
    },
    some_closure,
    thread_loop::ThreadLoop,
    types, Id,
};

#[derive(Clone)]
pub struct Params {
    seq: u32,
    pub pods: Vec<spa::pod::RawPodOwned>,
}

impl Params {
    fn new() -> Self {
        Params {
            seq: 0,
            pods: vec![],
        }
    }

    fn add(&mut self, seq: u32, pod: spa::pod::RawPodOwned) {
        if self.seq == seq {
            self.pods.push(pod)
        } else {
            self.pods.clear();
            self.seq = seq;
            self.pods.push(pod);
        }
    }
}

#[derive(Clone)]
pub struct ClientDetails {
    pub client: proxy::client::Client,
    pub props: Properties,
}

unsafe impl Send for ClientDetails {}
unsafe impl Sync for ClientDetails {}

#[derive(Clone)]
pub struct DeviceDetails {
    pub device: proxy::device::Device,
    pub props: Properties,
    pub params: HashMap<spa::param::ParamType, Params>,
}

unsafe impl Send for DeviceDetails {}
unsafe impl Sync for DeviceDetails {}

#[derive(Clone)]
pub struct ModuleDetails {
    pub module: proxy::module::Module,
    pub props: Properties,
}

unsafe impl Send for ModuleDetails {}
unsafe impl Sync for ModuleDetails {}

#[derive(Clone)]
pub struct NodeDetails {
    pub node: proxy::node::Node,
    pub props: Properties,
    pub params: HashMap<spa::param::ParamType, Params>,
}

unsafe impl Send for NodeDetails {}
unsafe impl Sync for NodeDetails {}

pub struct State {
    pub main_loop: ThreadLoop,
    ui_update: Arc<AtomicBool>,
    _context: Context,
    core: Core,
    registry: Registry,
    pub clients: Arc<Mutex<BTreeMap<Id, ClientDetails>>>,
    pub devices: Arc<Mutex<BTreeMap<Id, DeviceDetails>>>,
    pub modules: Arc<Mutex<BTreeMap<Id, ModuleDetails>>>,
    pub nodes: Arc<Mutex<BTreeMap<Id, NodeDetails>>>,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

impl State {
    pub fn new(name: &str, update: Arc<AtomicBool>) -> std::io::Result<Arc<State>> {
        pipewire::init();
        let mut props = Properties::new();
        props.set(keys::APP_NAME, name.to_string());

        let main_loop = ThreadLoop::new(&props).expect("main loop creation should not fail");
        let context = Context::new(main_loop.main_loop(), props)?;
        let core = context.connect(None)?;
        let registry = core.registry()?;

        let state = Arc::new(State {
            ui_update: update,
            main_loop,
            _context: context,
            core,
            registry,
            clients: Arc::new(Mutex::new(BTreeMap::new())),
            devices: Arc::new(Mutex::new(BTreeMap::new())),
            modules: Arc::new(Mutex::new(BTreeMap::new())),
            nodes: Arc::new(Mutex::new(BTreeMap::new())),
        });

        let pw_state = state.clone();

        let registry = &pw_state.registry;
        registry.add_listener(RegistryEvents {
            global: some_closure!([registry ^(state)] id, _perms, type_, version, _props, {
                let object = registry.bind(id, type_, version);

                match object {
                    Ok(object) => {
                        state.new_object(state, object);
                    }
                    Err(e) => {
                        if e.kind() != std::io::ErrorKind::Unsupported {
                            todo!("Send error {e} to UI");
                        }
                    }
                }
            }),
            ..Default::default()
        });

        Ok(state)
    }

    pub fn run(&self) {
        self.main_loop.run();
    }

    pub fn stop(&self) {
        self.main_loop.quit();
        self.core.disconnect();
    }

    fn new_object(&self, state: &Arc<Self>, object: Box<dyn proxy::HasProxy>) {
        match object.type_() {
            types::interface::CLIENT => {
                let client = object.downcast::<proxy::client::Client>().unwrap();

                client.add_listener(ClientEvents {
                    info: some_closure!([^(state)] info, {
                        state.client_info(info);
                    }),
                    ..Default::default()
                });

                let clients = &self.clients;
                client.proxy().add_listener(ProxyEvents {
                    bound_props: some_closure!([client ^(clients)] bound_id, props, {
                        clients.lock().unwrap().insert(
                            bound_id,
                            ClientDetails {
                                client,
                                props: props.clone(),
                            },
                        );
                    }),
                    removed: some_closure!([client ^(state)] {
                        state.client_removed(client);
                    }),
                    ..Default::default()
                });
            }
            types::interface::DEVICE => {
                let device = object.downcast::<proxy::device::Device>().unwrap();

                device
                    .subscribe_params(&[
                        spa::param::ParamType::EnumProfile,
                        spa::param::ParamType::Profile,
                        spa::param::ParamType::EnumRoute,
                        spa::param::ParamType::Route,
                    ])
                    .unwrap();

                device.add_listener(DeviceEvents {
                    info: some_closure!([^(state)] info, {
                        state.device_info(info);
                    }),
                    param: some_closure!([device ^(state)] seq, param_id, _index, _next, param_pod, {
                        state.device_param(&device, seq, param_id, param_pod);
                    }),
                });

                let devices = &self.devices;
                device.proxy().add_listener(ProxyEvents {
                    bound_props: some_closure!([device ^(devices)] bound_id, props, {
                        devices.lock().unwrap().insert(
                            bound_id,
                            DeviceDetails {
                                device,
                                props: props.clone(),
                                params: HashMap::new(),
                            },
                        );
                    }),
                    removed: some_closure!([device ^(state)] {
                        state.device_removed(device);
                    }),
                    ..Default::default()
                });
            }
            types::interface::MODULE => {
                let module = object.downcast::<proxy::module::Module>().unwrap();

                module.add_listener(ModuleEvents {
                    info: some_closure!([^(state)] info, {
                        state.module_info(info);
                    }),
                });

                let modules = &self.modules;
                module.proxy().add_listener(ProxyEvents {
                    bound_props: some_closure!([module ^(modules)] bound_id, props, {
                        modules.lock().unwrap().insert(
                            bound_id,
                            ModuleDetails {
                                module,
                                props: props.clone(),
                            },
                        );
                    }),
                    removed: some_closure!([module ^(state)] {
                        state.module_removed(module);
                    }),
                    ..Default::default()
                });
            }
            types::interface::NODE => {
                let node = object.downcast::<proxy::node::Node>().unwrap();

                node.subscribe_params(&[
                    spa::param::ParamType::PropInfo,
                    spa::param::ParamType::Props,
                    spa::param::ParamType::EnumFormat,
                    spa::param::ParamType::Format,
                    spa::param::ParamType::Buffers,
                    spa::param::ParamType::Meta,
                ])
                .unwrap();

                node.add_listener(NodeEvents {
                    info: some_closure!([^(state)] info, {
                        state.node_info(info);
                    }),
                    param: some_closure!([node ^(state)] seq, param_id, _index, _next, param_pod, {
                        state.node_param(&node, seq, param_id, param_pod);
                    }),
                });

                let nodes = &self.nodes;
                node.proxy().add_listener(ProxyEvents {
                    bound_props: some_closure!([node ^(nodes)] bound_id, props, {
                        nodes.lock().unwrap().insert(
                            bound_id,
                            NodeDetails {
                                node,
                                props: props.clone(),
                                params: HashMap::new(),
                            },
                        );
                    }),
                    removed: some_closure!([node ^(state)] {
                        state.node_removed(node);
                    }),
                    ..Default::default()
                });
            }
            _ => {}
        }

        self.ui_update.store(true, Ordering::Relaxed);
    }

    fn client_info(&self, info: &ClientInfo) {
        if let Some((_, entry)) = self
            .clients
            .lock()
            .unwrap()
            .iter_mut()
            .find(|(_, e)| e.client.proxy().bound_id() == Some(info.id))
        {
            entry.props = info.props.clone();
            self.ui_update.store(true, Ordering::Relaxed);
        }
    }

    fn client_removed(&self, client: proxy::client::Client) {
        let _ = self
            .clients
            .lock()
            .unwrap()
            .remove(&client.proxy().bound_id().unwrap());
        self.ui_update.store(true, Ordering::Relaxed);
    }

    fn device_info(&self, info: &DeviceInfo) {
        if let Some((_, entry)) = self
            .devices
            .lock()
            .unwrap()
            .iter_mut()
            .find(|(_, e)| e.device.proxy().bound_id() == Some(info.id))
        {
            entry.props = info.props.clone();
            self.ui_update.store(true, Ordering::Relaxed);
        }
    }

    fn device_param(
        &self,
        device: &proxy::device::Device,
        seq: u32,
        param_id: spa::param::ParamType,
        pod: &spa::pod::RawPodOwned,
    ) {
        let mut devices = self.devices.lock().unwrap();
        if let Some((_, entry)) = devices
            .iter_mut()
            .find(|(_, e)| e.device.proxy().bound_id() == Some(device.proxy().bound_id().unwrap()))
        {
            match entry.params.get_mut(&param_id) {
                Some(p) => p.add(seq, pod.clone()),
                None => {
                    let mut p = Params::new();
                    p.add(seq, pod.clone());
                    entry.params.insert(param_id, p);
                }
            }
        };
    }

    fn device_removed(&self, device: proxy::device::Device) {
        let _ = self
            .devices
            .lock()
            .unwrap()
            .remove(&device.proxy().bound_id().unwrap());
        self.ui_update.store(true, Ordering::Relaxed);
    }

    fn module_info(&self, info: &ModuleInfo) {
        if let Some((_, entry)) = self
            .modules
            .lock()
            .unwrap()
            .iter_mut()
            .find(|(_, e)| e.module.proxy().bound_id() == Some(info.id))
        {
            entry.props = info.props.clone();
            self.ui_update.store(true, Ordering::Relaxed);
        }
    }

    fn module_removed(&self, module: proxy::module::Module) {
        let _ = self
            .modules
            .lock()
            .unwrap()
            .remove(&module.proxy().bound_id().unwrap());
        self.ui_update.store(true, Ordering::Relaxed);
    }

    fn node_info(&self, info: &NodeInfo) {
        if let Some((_, entry)) = self
            .nodes
            .lock()
            .unwrap()
            .iter_mut()
            .find(|(_, e)| e.node.proxy().bound_id() == Some(info.id))
        {
            entry.props = info.props.clone();
            self.ui_update.store(true, Ordering::Relaxed);
        }
    }

    fn node_param(
        &self,
        node: &proxy::node::Node,
        seq: u32,
        param_id: spa::param::ParamType,
        pod: &spa::pod::RawPodOwned,
    ) {
        let mut nodes = self.nodes.lock().unwrap();
        if let Some((_, entry)) = nodes
            .iter_mut()
            .find(|(_, e)| e.node.proxy().bound_id() == Some(node.proxy().bound_id().unwrap()))
        {
            match entry.params.get_mut(&param_id) {
                Some(p) => p.add(seq, pod.clone()),
                None => {
                    let mut p = Params::new();
                    p.add(seq, pod.clone());
                    entry.params.insert(param_id, p);
                }
            }
        };
    }

    fn node_removed(&self, node: proxy::node::Node) {
        let _ = self
            .nodes
            .lock()
            .unwrap()
            .remove(&node.proxy().bound_id().unwrap());
        self.ui_update.store(true, Ordering::Relaxed);
    }
}
