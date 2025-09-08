// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc, RwLock,
    },
};

use pipewire_native::{
    self as pipewire, closure,
    context::Context,
    core::Core,
    main_loop::MainLoop,
    properties::Properties,
    proxy::{
        client::Client,
        device::Device,
        factory::Factory,
        link::Link,
        metadata::Metadata,
        module::Module,
        node::Node,
        port::Port,
        registry::{Registry, RegistryEvents},
        HasProxy, ProxyEvents,
    },
    some_closure, types, Id,
};

#[allow(unused)]
struct TestContext {
    runtime_dir: tempfile::TempDir,
    pipewire: std::process::Child,
}

fn start_pipewire() -> TestContext {
    let runtime_dir = tempfile::tempdir().unwrap();

    std::env::set_var("PIPEWIRE_RUNTIME_DIR", runtime_dir.path());

    let pipewire = std::process::Command::new("pipewire").spawn().unwrap();

    std::thread::sleep(std::time::Duration::from_millis(500));

    TestContext {
        runtime_dir,
        pipewire,
    }
}

#[derive(Clone)]
struct Objects {
    map: Arc<RwLock<HashMap<Id, Box<dyn HasProxy>>>>,
    seq: Arc<AtomicU32>,
    input_node: Arc<RwLock<Option<Box<dyn HasProxy>>>>,
    output_node: Arc<RwLock<Option<Box<dyn HasProxy>>>>,
    link: Arc<RwLock<Option<Box<dyn HasProxy>>>>,
}

impl Objects {
    fn input_id(&self) -> Option<u32> {
        self.input_node
            .read()
            .unwrap()
            .as_ref()
            .and_then(|n| n.downcast_proxy::<Node>())
            .and_then(|p| p.bound_id())
    }

    fn output_id(&self) -> Option<u32> {
        self.output_node
            .read()
            .unwrap()
            .as_ref()
            .and_then(|n| n.downcast_proxy::<Node>())
            .and_then(|p| p.bound_id())
    }
}

unsafe impl Send for Objects {}

fn create_nodes(core: &Core, objects: &Objects) {
    if objects.output_node.read().unwrap().is_none() {
        let mut props = Properties::new();

        props.set(
            "library.name",
            "audiotestsrc/libspa-audiotestsrc".to_string(),
        );
        props.set("factory.name", "audiotestsrc".to_string());
        props.set("node.name", "testsrc".to_string());

        let object = core
            .create_object("spa-node-factory", types::interface::NODE, 3, &props)
            .unwrap();

        let proxy = object.downcast_proxy::<Node>().unwrap();

        proxy.add_listener(ProxyEvents {
            bound_props: some_closure!([core ^(objects)] _id, _props, {
                create_link(&core, objects);
            }),
            ..Default::default()
        });

        objects.output_node.write().unwrap().replace(object);

        let _ = core.sync();
    }

    if objects.input_node.read().unwrap().is_none() {
        let mut props = Properties::new();

        props.set("library.name", "support/libspa-support".to_string());
        props.set("factory.name", "support.null-audio-sink".to_string());
        props.set("node.name", "nullsink".to_string());

        let object = core
            .create_object("spa-node-factory", types::interface::NODE, 3, &props)
            .unwrap();

        let proxy = object.downcast_proxy::<Node>().unwrap();

        proxy.add_listener(ProxyEvents {
            bound_props: some_closure!([core ^(objects)] _id, _props, {
                create_link(&core, objects);
            }),
            ..Default::default()
        });

        objects.input_node.write().unwrap().replace(object);

        let _ = core.sync();
    }
}

fn create_link(core: &Core, objects: &Objects) {
    if objects.link.read().unwrap().is_some() {
        return;
    }

    let (input_node_id, output_node_id) = match (objects.input_id(), objects.output_id()) {
        (Some(i), Some(o)) => (i, o),
        _ => return,
    };

    println!("We can create link");

    let mut props = Properties::new();

    props.set("link.input.node", format!("{}", input_node_id));
    props.set("link.output.node", format!("{}", output_node_id));

    objects.link.write().unwrap().replace(
        core.create_object("link-factory", types::interface::LINK, 3, &props)
            .unwrap(),
    );

    let _ = core.sync();
}

fn destroy_nodes(registry: &Registry, objects: &Objects) {
    let _ = objects.input_node.write().unwrap().take().unwrap();
    let _ = objects.output_node.write().unwrap().take().unwrap();
    let link = objects.link.write().unwrap().take();

    if let Some(link) = link {
        // Link creation might fail if the PipeWire install doesn't have audiotestsrc, so don't
        // make this fatal.
        let _ = registry.destroy(
            link.downcast_proxy::<Link>()
                .and_then(|p| p.bound_id())
                .unwrap(),
        );
    };

    if let Some(id) = objects.input_id() {
        let _ = registry.destroy(id);
    }

    if let Some(id) = objects.output_id() {
        let _ = registry.destroy(id);
    }
}

#[test]
fn test_lib() {
    let _test_context = start_pipewire();

    pipewire::init();

    let objects = Objects {
        map: Arc::new(RwLock::new(HashMap::new())),
        seq: Arc::new(AtomicU32::new(0)),
        input_node: Arc::new(RwLock::new(None)),
        output_node: Arc::new(RwLock::new(None)),
        link: Arc::new(RwLock::new(None)),
    };

    let v = vec![("loop.name".to_string(), "pw-main-loop".to_string())];
    let main_loop = MainLoop::new(&Properties::new_vec(v)).unwrap();

    let context =
        Context::new(&main_loop, Properties::new()).expect("Context creation should not fail");

    let core = context.connect(None).unwrap();

    core.proxy().add_listener(ProxyEvents {
        done: some_closure!([core ^(objects)] seq, {
            if seq == objects.seq.load(Ordering::Relaxed) {
                create_nodes(&core, objects);
            }
        }),
        error: some_closure!([] seq, res, msg, {
            unreachable!("Error: {seq} {res} {msg}");
        }),
        destroy: some_closure!([^(objects)] {
            println!("core destroyed, clearing objects");
            objects.map.write().unwrap().clear();
        }),
        ..Default::default()
    });

    let registry = core.registry().unwrap();

    let mut timer_src = main_loop
        .add_timer(
            closure!([core, main_loop, registry ^(objects)] _expirations, {
                destroy_nodes(&registry, objects);
                assert!(objects.map.read().unwrap().len() > 1);
                core.disconnect();
                assert_eq!(objects.map.read().unwrap().len(), 0);
                main_loop.quit();
            }),
        )
        .unwrap();

    let timeout = libc::timespec {
        tv_sec: 0,
        tv_nsec: 200_000_000,
    };
    let res = main_loop.update_timer(&mut timer_src, &timeout, None, false);
    assert!(res.is_ok());

    registry.add_listener(RegistryEvents {
        global: some_closure!([registry ^(objects)] id, perms, type_, version, props, {
            println!("new global id {id}: {type_}/{version} ({perms}): {{ {props:?} }}");

            let object = match type_ {
                types::interface::CLIENT => {
                    let client = registry.bind(id, type_, version).unwrap();
                    let proxy = client.downcast_proxy::<Client>().unwrap();

                    proxy.add_listener(ProxyEvents {
                        removed: some_closure!([proxy ^(objects)] {
                            objects.map.write().unwrap().remove(&proxy.id());
                        }),
                        ..Default::default()
                    });

                    client
                }
                types::interface::DEVICE => {
                    let device = registry.bind(id, type_, version).unwrap();
                    let proxy = device.downcast_proxy::<Device>().unwrap();

                    proxy.add_listener(ProxyEvents {
                        removed: some_closure!([proxy ^(objects)] {
                            objects.map.write().unwrap().remove(&proxy.id());
                        }),
                        ..Default::default()
                    });

                    device
                }
                types::interface::FACTORY => {
                    let factory = registry.bind(id, type_, version).unwrap();
                    let proxy = factory.downcast_proxy::<Factory>().unwrap();

                    proxy.add_listener(ProxyEvents {
                        removed: some_closure!([proxy ^(objects)] {
                            objects.map.write().unwrap().remove(&proxy.id());
                        }),
                        ..Default::default()
                    });

                    factory
                }
                types::interface::LINK => {
                    let link = registry.bind(id, type_, version).unwrap();
                    let proxy = link.downcast_proxy::<Link>().unwrap();

                    proxy.add_listener(ProxyEvents {
                        removed: some_closure!([proxy ^(objects)] {
                            objects.map.write().unwrap().remove(&proxy.id());
                        }),
                        ..Default::default()
                    });

                    link
                }
                types::interface::METADATA => {
                    let metadata = registry.bind(id, type_, version).unwrap();
                    let proxy = metadata.downcast_proxy::<Metadata>().unwrap();

                    proxy.add_listener(ProxyEvents {
                        removed: some_closure!([proxy ^(objects)] {
                            objects.map.write().unwrap().remove(&proxy.id());
                        }),
                        ..Default::default()
                    });

                    metadata
                }
                types::interface::MODULE => {
                    let module = registry.bind(id, type_, version).unwrap();
                    let proxy = module.downcast_proxy::<Module>().unwrap();

                    proxy.add_listener(ProxyEvents {
                        removed: some_closure!([proxy ^(objects)] {
                            objects.map.write().unwrap().remove(&proxy.id());
                        }),
                        ..Default::default()
                    });

                    module
                }
                types::interface::NODE => {
                    match props.get("node.name") {
                        // We already have proxies for these
                        Some("testsrc") => return,
                        Some("nullsink") => return,
                        _ => (),
                    };

                    let node = registry.bind(id, type_, version).unwrap();
                    let proxy = node.downcast_proxy::<Node>().unwrap();

                    proxy.add_listener(ProxyEvents {
                        removed: some_closure!([proxy ^(objects)] {
                            objects.map.write().unwrap().remove(&proxy.id());
                        }),
                        ..Default::default()
                    });

                    node
                }
                types::interface::PORT => {
                    let port = registry.bind(id, type_, version).unwrap();
                    let proxy = port.downcast_proxy::<Port>().unwrap();

                    proxy.add_listener(ProxyEvents {
                        removed: some_closure!([proxy ^(objects)] {
                            objects.map.write().unwrap().remove(&proxy.id());
                        }),
                        ..Default::default()
                    });

                    port
                }
                _ => return,
            };

            objects.map.write().unwrap().insert(id, object);
        }),
        global_remove: some_closure!([^(objects)] id, {
            println!("global {id} removed");
            let _ = objects.map.write().unwrap().remove(&id);
        }),
    });

    let seq = core.sync().unwrap();
    objects.seq.store(seq, Ordering::Relaxed);

    main_loop.run();

    assert_eq!(objects.map.read().unwrap().len(), 0);
    assert!(objects.input_node.read().unwrap().is_none());
    assert!(objects.output_node.read().unwrap().is_none());
}
