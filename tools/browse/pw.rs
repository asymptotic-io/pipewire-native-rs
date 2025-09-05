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
        factory::{FactoryEvents, FactoryInfo},
        link::{LinkEvents, LinkInfo},
        metadata::{Metadata, MetadataEvents},
        module::{ModuleEvents, ModuleInfo},
        node::{NodeEvents, NodeInfo},
        port::{PortDirection, PortEvents, PortInfo},
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
pub struct FactoryDetails {
    pub factory: proxy::factory::Factory,
    pub props: Properties,
}

unsafe impl Send for FactoryDetails {}
unsafe impl Sync for FactoryDetails {}

#[derive(Clone)]
pub struct LinkDetails {
    pub link: proxy::link::Link,
    pub props: Properties,
}

unsafe impl Send for LinkDetails {}
unsafe impl Sync for LinkDetails {}

#[derive(Clone)]
pub struct MetadataDetails {
    pub metadata: proxy::metadata::Metadata,
    pub props: Properties,
}

unsafe impl Send for MetadataDetails {}
unsafe impl Sync for MetadataDetails {}

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
    pub max_input_ports: u32,
    pub max_output_ports: u32,
    pub props: Properties,
    pub params: HashMap<spa::param::ParamType, Params>,
}

unsafe impl Send for NodeDetails {}
unsafe impl Sync for NodeDetails {}

#[derive(Clone)]
pub struct PortDetails {
    pub port: proxy::port::Port,
    pub direction: proxy::port::PortDirection,
    pub props: Properties,
    pub params: HashMap<spa::param::ParamType, Params>,
}

unsafe impl Send for PortDetails {}
unsafe impl Sync for PortDetails {}

pub struct State {
    pub main_loop: ThreadLoop,
    ui_update: Arc<AtomicBool>,
    _context: Context,
    pub core: Core,
    pub registry: Registry,
    pub clients: Arc<Mutex<BTreeMap<Id, ClientDetails>>>,
    pub devices: Arc<Mutex<BTreeMap<Id, DeviceDetails>>>,
    pub factories: Arc<Mutex<BTreeMap<Id, FactoryDetails>>>,
    pub links: Arc<Mutex<BTreeMap<Id, LinkDetails>>>,
    pub metadata: Arc<Mutex<BTreeMap<Id, MetadataDetails>>>,
    pub modules: Arc<Mutex<BTreeMap<Id, ModuleDetails>>>,
    pub nodes: Arc<Mutex<BTreeMap<Id, NodeDetails>>>,
    pub ports: Arc<Mutex<BTreeMap<Id, PortDetails>>>,
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
            factories: Arc::new(Mutex::new(BTreeMap::new())),
            links: Arc::new(Mutex::new(BTreeMap::new())),
            metadata: Arc::new(Mutex::new(BTreeMap::new())),
            modules: Arc::new(Mutex::new(BTreeMap::new())),
            nodes: Arc::new(Mutex::new(BTreeMap::new())),
            ports: Arc::new(Mutex::new(BTreeMap::new())),
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

    pub fn link_nodes(&self, node1: &NodeDetails, node2: &NodeDetails) {
        let (output_node, input_node) = if node1.max_output_ports > 0 {
            (node1, node2)
        } else {
            (node2, node1)
        };

        let mut props = Properties::new();

        props.set(
            keys::LINK_INPUT_NODE,
            format!("{}", input_node.node.proxy().bound_id().unwrap()),
        );
        props.set(
            keys::LINK_OUTPUT_NODE,
            format!("{}", output_node.node.proxy().bound_id().unwrap()),
        );

        self.core
            .create_object("link-factory", types::interface::LINK, 3, &props)
            .unwrap();
    }

    pub fn link_ports(&self, port1: &PortDetails, port2: &PortDetails) {
        let (output_port, input_port) = if port1.direction == PortDirection::Output {
            (port1, port2)
        } else {
            (port2, port1)
        };

        let mut props = Properties::new();

        props.set(
            keys::LINK_INPUT_PORT,
            format!("{}", input_port.port.proxy().bound_id().unwrap()),
        );
        props.set(
            keys::LINK_OUTPUT_PORT,
            format!("{}", output_port.port.proxy().bound_id().unwrap()),
        );

        self.core
            .create_object("link-factory", types::interface::LINK, 3, &props)
            .unwrap();
    }

    fn node_name(&self, id: Id) -> Option<String> {
        self.nodes
            .lock()
            .unwrap()
            .get(&id)
            .map(|n| n.props.get("node.name").unwrap_or("unknown").to_string())
    }

    fn port_name(&self, id: Id) -> Option<String> {
        self.ports
            .lock()
            .unwrap()
            .get(&id)
            .map(|n| n.props.get("port.name").unwrap_or("unknown").to_string())
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
            types::interface::FACTORY => {
                let factory = object.downcast::<proxy::factory::Factory>().unwrap();

                factory.add_listener(FactoryEvents {
                    info: some_closure!([^(state)] info, {
                        state.factory_info(info);
                    }),
                });

                let factories = &self.factories;
                factory.proxy().add_listener(ProxyEvents {
                    bound_props: some_closure!([factory ^(factories)] bound_id, props, {
                        factories.lock().unwrap().insert(
                            bound_id,
                            FactoryDetails {
                                factory,
                                props: props.clone(),
                            },
                        );
                    }),
                    removed: some_closure!([factory ^(state)] {
                        state.factory_removed(factory);
                    }),
                    ..Default::default()
                });
            }
            types::interface::LINK => {
                let link = object.downcast::<proxy::link::Link>().unwrap();

                link.add_listener(LinkEvents {
                    info: some_closure!([^(state)] info, {
                        state.link_info(info);
                    }),
                });

                let links = &self.links;
                link.proxy().add_listener(ProxyEvents {
                    bound_props: some_closure!([link ^(links)] bound_id, props, {
                        links.lock().unwrap().insert(
                            bound_id,
                            LinkDetails {
                                link,
                                props: props.clone(),
                            },
                        );
                    }),
                    removed: some_closure!([link ^(state)] {
                        state.link_removed(link);
                    }),
                    ..Default::default()
                });
            }
            types::interface::METADATA => {
                let metadata = object.downcast::<proxy::metadata::Metadata>().unwrap();

                metadata.add_listener(MetadataEvents {
                    property: some_closure!([metadata ^(state)] subject, key, type_, value, {
                        state.metadata_property(&metadata, subject, key, type_, value);
                    }),
                });

                let metadatas = &self.metadata;
                metadata.proxy().add_listener(ProxyEvents {
                    bound_props: some_closure!([metadata ^(metadatas)] bound_id, props, {
                        metadatas.lock().unwrap().insert(
                            bound_id,
                            MetadataDetails {
                                metadata,
                                props: props.clone(),
                            },
                        );
                    }),
                    removed: some_closure!([metadata ^(state)] {
                        state.metadata_removed(metadata);
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
                    // TODO: IO-related metadata
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
                                max_input_ports: 0,
                                max_output_ports: 0,
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
            types::interface::PORT => {
                let port = object.downcast::<proxy::port::Port>().unwrap();

                port.subscribe_params(&[
                    spa::param::ParamType::EnumFormat,
                    spa::param::ParamType::Format,
                    spa::param::ParamType::Buffers,
                    spa::param::ParamType::Meta,
                    // TODO: IO-related metadata and Latency
                ])
                .unwrap();

                port.add_listener(PortEvents {
                    info: some_closure!([^(state)] info, {
                        state.port_info(info);
                    }),
                    param: some_closure!([port ^(state)] seq, param_id, _index, _next, param_pod, {
                        state.port_param(&port, seq, param_id, param_pod);
                    }),
                });

                let ports = &self.ports;
                port.proxy().add_listener(ProxyEvents {
                    bound_props: some_closure!([port ^(ports)] bound_id, props, {
                        ports.lock().unwrap().insert(
                            bound_id,
                            PortDetails {
                                port,
                                direction: proxy::port::PortDirection::Input,
                                props: props.clone(),
                                params: HashMap::new(),
                            },
                        );
                    }),
                    removed: some_closure!([port ^(state)] {
                        state.port_removed(port);
                    }),
                    ..Default::default()
                });
            }
            _ => {}
        }

        self.ui_update.store(true, Ordering::Relaxed);
    }

    fn client_info(&self, info: &ClientInfo) {
        if let Some(entry) = self.clients.lock().unwrap().get_mut(&info.id) {
            entry.props.merge(info.props);
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
        if let Some(entry) = self.devices.lock().unwrap().get_mut(&info.id) {
            entry.props.merge(info.props);
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
        if let Some(entry) = devices.get_mut(&device.proxy().bound_id().unwrap()) {
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

    fn factory_info(&self, info: &FactoryInfo) {
        if let Some(entry) = self.factories.lock().unwrap().get_mut(&info.id) {
            entry.props.merge(info.props);
            self.ui_update.store(true, Ordering::Relaxed);
        }
    }

    fn factory_removed(&self, factory: proxy::factory::Factory) {
        let _ = self
            .factories
            .lock()
            .unwrap()
            .remove(&factory.proxy().bound_id().unwrap());
        self.ui_update.store(true, Ordering::Relaxed);
    }

    fn link_info(&self, info: &LinkInfo) {
        if let Some(entry) = self.links.lock().unwrap().get_mut(&info.id) {
            entry.props.merge(info.props);

            // Add a couple of props to make names look nicer
            if let Some(name) = self.node_name(info.output_node_id) {
                entry.props.set("pw-browse.output-node", name);
            }
            if let Some(name) = self.port_name(info.output_port_id) {
                entry.props.set("pw-browse.output-port", name);
            }
            if let Some(name) = self.node_name(info.input_node_id) {
                entry.props.set("pw-browse.input-node", name);
            }
            if let Some(name) = self.port_name(info.input_port_id) {
                entry.props.set("pw-browse.input-port", name);
            }

            self.ui_update.store(true, Ordering::Relaxed);
        }
    }

    fn link_removed(&self, link: proxy::link::Link) {
        let _ = self
            .links
            .lock()
            .unwrap()
            .remove(&link.proxy().bound_id().unwrap());
        self.ui_update.store(true, Ordering::Relaxed);
    }

    fn module_info(&self, info: &ModuleInfo) {
        if let Some(entry) = self.modules.lock().unwrap().get_mut(&info.id) {
            entry.props.merge(info.props);
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

    fn metadata_property(
        &self,
        metadata: &Metadata,
        subject: Id,
        key: Option<&str>,
        _type_: Option<&str>,
        value: Option<&str>,
    ) {
        if let Some(entry) = self
            .metadata
            .lock()
            .unwrap()
            .get_mut(&metadata.proxy().bound_id().unwrap())
        {
            if let Some(key) = key {
                let key = format!("{subject}: {key}");

                if let Some(value) = value {
                    // (key, value) was set on subject
                    entry.props.set(key.as_str(), value.to_string());
                } else {
                    // (key, value) was unset on subject
                    entry.props.unset(key.as_str());
                }
            } else {
                // All keys were removed on subject
                let key_prefix = format!("{subject}/");
                let keys: Vec<String> = entry
                    .props
                    .iter()
                    .filter_map(|(k, _)| {
                        if k.starts_with(key_prefix.as_str()) {
                            Some(k.to_string())
                        } else {
                            None
                        }
                    })
                    .collect();

                for key in keys {
                    if key.starts_with(key_prefix.as_str()) {
                        entry.props.unset(key.as_str());
                    }
                }
            }

            self.ui_update.store(true, Ordering::Relaxed);
        }
    }

    fn metadata_removed(&self, metadata: proxy::metadata::Metadata) {
        let _ = self
            .metadata
            .lock()
            .unwrap()
            .remove(&metadata.proxy().bound_id().unwrap());
        self.ui_update.store(true, Ordering::Relaxed);
    }

    fn node_info(&self, info: &NodeInfo) {
        if let Some(entry) = self.nodes.lock().unwrap().get_mut(&info.id) {
            entry.max_input_ports = info.max_input_ports;
            entry.max_output_ports = info.max_output_ports;
            entry.props.merge(info.props);
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
        if let Some(entry) = nodes.get_mut(&node.proxy().bound_id().unwrap()) {
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

    fn port_info(&self, info: &PortInfo) {
        if let Some(entry) = self.ports.lock().unwrap().get_mut(&info.id) {
            entry.direction = info.direction;
            entry.props.merge(info.props);

            if let Some(name) = entry
                .props
                .get_u32("node.id")
                .and_then(|node_id| self.node_name(node_id))
            {
                entry.props.set("pw-browse.node", name);
            }

            self.ui_update.store(true, Ordering::Relaxed);
        }
    }

    fn port_param(
        &self,
        port: &proxy::port::Port,
        seq: u32,
        param_id: spa::param::ParamType,
        pod: &spa::pod::RawPodOwned,
    ) {
        let mut ports = self.ports.lock().unwrap();
        if let Some(entry) = ports.get_mut(&port.proxy().bound_id().unwrap()) {
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

    fn port_removed(&self, port: proxy::port::Port) {
        let _ = self
            .ports
            .lock()
            .unwrap()
            .remove(&port.proxy().bound_id().unwrap());
        self.ui_update.store(true, Ordering::Relaxed);
    }
}
