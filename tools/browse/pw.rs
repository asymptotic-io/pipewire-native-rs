// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    collections::BTreeMap,
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
        registry::{Registry, RegistryEvents},
        HasProxy, ProxyEvents,
    },
    some_closure,
    thread_loop::ThreadLoop,
    types, Id,
};

#[derive(Clone)]
pub struct ClientDetails {
    pub client: proxy::client::Client,
    pub props: Properties,
}

#[derive(Clone)]
pub struct DeviceDetails {
    pub device: proxy::device::Device,
    pub props: Properties,
    pub params: Vec<(spa::param::ParamType, spa::pod::RawPodOwned)>,
}

#[derive(Clone)]
pub struct ModuleDetails {
    pub module: proxy::module::Module,
    pub props: Properties,
}

pub struct State {
    pub main_loop: ThreadLoop,
    ui_update: Arc<AtomicBool>,
    _context: Context,
    core: Core,
    registry: Registry,
    pub clients: Arc<Mutex<BTreeMap<Id, ClientDetails>>>,
    pub devices: Arc<Mutex<BTreeMap<Id, DeviceDetails>>>,
    pub modules: Arc<Mutex<BTreeMap<Id, ModuleDetails>>>,
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
                    param: some_closure!([device ^(state)] _seq, param_type, _index, _next, param_pod, {
                        state.device_param(&device, param_type, param_pod);
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
                                params: vec![],
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
        param_type: spa::param::ParamType,
        param_pod: &spa::pod::RawPodOwned,
    ) {
        let mut devices = self.devices.lock().unwrap();
        if let Some((_, entry)) = devices
            .iter_mut()
            .find(|(_, e)| e.device.proxy().bound_id() == Some(device.proxy().bound_id().unwrap()))
        {
            entry.params.push((param_type, param_pod.clone()));
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
}
