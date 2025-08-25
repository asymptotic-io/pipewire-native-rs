// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
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

pub struct State {
    pub main_loop: ThreadLoop,
    ui_update: Arc<AtomicBool>,
    _context: Context,
    core: Core,
    registry: Registry,
    pub clients: RwLock<BTreeMap<Id, (proxy::client::Client, Properties)>>,
    pub devices: RwLock<BTreeMap<Id, (proxy::device::Device, Properties)>>,
    pub modules: RwLock<BTreeMap<Id, (proxy::module::Module, Properties)>>,
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
            clients: RwLock::new(BTreeMap::new()),
            devices: RwLock::new(BTreeMap::new()),
            modules: RwLock::new(BTreeMap::new()),
        });

        let pw_state = state.clone();

        let registry = &pw_state.registry;
        registry.add_listener(RegistryEvents {
            global: some_closure!([registry ^(state)] id, _perms, type_, version, props, {
                let object = registry.bind(id, type_, version);

                match object {
                    Ok(object) => {
                        state.new_object(state, object, props);
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

    fn new_object(&self, state: &Arc<Self>, object: Box<dyn proxy::HasProxy>, props: &Properties) {
        match object.type_() {
            types::interface::CLIENT => {
                let client = object.downcast::<proxy::client::Client>().unwrap();

                client.add_listener(ClientEvents {
                    info: some_closure!([^(state)] info, {
                        state.client_info(info);
                    }),
                    ..Default::default()
                });

                client.proxy().add_listener(ProxyEvents {
                    removed: some_closure!([client ^(state)] {
                        state.client_removed(client);
                    }),
                    ..Default::default()
                });

                self.clients
                    .write()
                    .unwrap()
                    .insert(client.proxy().id(), (client, props.clone()));
            }
            types::interface::DEVICE => {
                let device = object.downcast::<proxy::device::Device>().unwrap();

                device.add_listener(DeviceEvents {
                    info: some_closure!([^(state)] info, {
                        state.device_info(info);
                    }),
                    ..Default::default()
                });

                device.proxy().add_listener(ProxyEvents {
                    removed: some_closure!([device ^(state)] {
                        state.device_removed(device);
                    }),
                    ..Default::default()
                });

                self.devices
                    .write()
                    .unwrap()
                    .insert(device.proxy().id(), (device, props.clone()));
            }
            types::interface::MODULE => {
                let module = object.downcast::<proxy::module::Module>().unwrap();

                module.add_listener(ModuleEvents {
                    info: some_closure!([^(state)] info, {
                        state.module_info(info);
                    }),
                });

                module.proxy().add_listener(ProxyEvents {
                    removed: some_closure!([module ^(state)] {
                        state.module_removed(module);
                    }),
                    ..Default::default()
                });

                self.modules
                    .write()
                    .unwrap()
                    .insert(module.proxy().id(), (module, props.clone()));
            }
            _ => {}
        }

        self.ui_update.store(true, Ordering::Relaxed);
    }

    fn client_info(&self, info: &ClientInfo) {
        if let Some((_, entry)) = self
            .clients
            .write()
            .unwrap()
            .iter_mut()
            .find(|(_, e)| e.0.proxy().bound_id() == Some(info.id))
        {
            entry.1 = info.props.clone();
            self.ui_update.store(true, Ordering::Relaxed);
        }
    }

    fn client_removed(&self, client: proxy::client::Client) {
        let _ = self.clients.write().unwrap().remove(&client.proxy().id());
        self.ui_update.store(true, Ordering::Relaxed);
    }

    fn device_info(&self, info: &DeviceInfo) {
        if let Some((_, entry)) = self
            .devices
            .write()
            .unwrap()
            .iter_mut()
            .find(|(_, e)| e.0.proxy().bound_id() == Some(info.id))
        {
            entry.1 = info.props.clone();
            self.ui_update.store(true, Ordering::Relaxed);
        }
    }

    fn device_removed(&self, device: proxy::device::Device) {
        let _ = self.devices.write().unwrap().remove(&device.proxy().id());
        self.ui_update.store(true, Ordering::Relaxed);
    }

    fn module_info(&self, info: &ModuleInfo) {
        if let Some((_, entry)) = self
            .modules
            .write()
            .unwrap()
            .iter_mut()
            .find(|(_, e)| e.0.proxy().bound_id() == Some(info.id))
        {
            entry.1 = info.props.clone();
            self.ui_update.store(true, Ordering::Relaxed);
        }
    }

    fn module_removed(&self, module: proxy::module::Module) {
        let _ = self.modules.write().unwrap().remove(&module.proxy().id());
        self.ui_update.store(true, Ordering::Relaxed);
    }
}
