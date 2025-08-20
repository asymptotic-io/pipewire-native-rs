// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{cell::RefCell, collections::HashMap, sync::Arc};

use pipewire::{
    self,
    context::Context,
    core::Core,
    keys,
    main_loop::MainLoop,
    properties::Properties,
    proxy::{
        self,
        client::{ClientEvents, ClientInfo},
        registry::{Registry, RegistryEvents},
        HasProxy, ProxyEvents,
    },
    some_closure, types, Id,
};

pub struct State {
    pub main_loop: MainLoop,
    _context: Context,
    core: Core,
    registry: Registry,
    pub clients: RefCell<HashMap<Id, (proxy::client::Client, Properties)>>,
}

unsafe impl Send for State {}
unsafe impl Sync for State {}

impl State {
    pub fn new(name: &str) -> std::io::Result<Arc<State>> {
        pipewire::init();
        let mut props = Properties::new();
        props.set(keys::APP_NAME, name.to_string());

        let main_loop = MainLoop::new(&props).expect("main loop creation should not fail");
        let context = Context::new(&main_loop, props)?;
        let core = context.connect(None)?;
        let registry = core.registry()?;

        let state = Arc::new(State {
            main_loop,
            _context: context,
            core,
            registry,
            clients: RefCell::new(HashMap::new()),
        });

        let pw_state = state.clone();

        let registry = &pw_state.registry;
        registry.add_listener(RegistryEvents {
            global: some_closure!([registry ^(state)] id, _perms, type_, version, props, {
                if type_ != types::interface::CLIENT {
                    return;
                }

                let object = registry.bind(id, type_, version);

                match object {
                    Ok(object) => {
                        state.new_object(state, object, props);
                    }
                    Err(e) => todo!("Send error {e} to UI"),
                }
            }),
            ..Default::default()
        });

        Ok(state)
    }

    pub fn run(&self) -> std::thread::JoinHandle<()> {
        let main_loop = self.main_loop.clone();

        std::thread::spawn(move || {
            main_loop.run();
        })
    }

    pub fn quit(&self) {
        self.main_loop.quit();
        self.core.disconnect();
    }
    fn new_object(&self, state: &Arc<Self>, object: Box<dyn proxy::HasProxy>, props: &Properties) {
        if object.type_() == types::interface::CLIENT {
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
                .borrow_mut()
                .insert(client.proxy().id(), (client, props.clone()));
        }
    }

    fn client_info(&self, info: &ClientInfo) {
        if let Some((_, entry)) = self
            .clients
            .borrow_mut()
            .iter_mut()
            .find(|(_, e)| e.0.proxy().bound_id() == Some(info.id))
        {
            entry.1 = info.props.clone();
        }
    }

    fn client_removed(&self, client: proxy::client::Client) {
        let _ = self.clients.borrow_mut().remove(&client.proxy().id());
    }
}
