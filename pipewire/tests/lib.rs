// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{cell::RefCell, collections::HashMap, sync::Arc};
use tempfile;

use pipewire_native::{
    self as pipewire, closure,
    context::Context,
    main_loop::MainLoop,
    properties::Properties,
    proxy::{client::Client, registry::RegistryEvents, HasProxy, ProxyEvents},
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

    std::thread::sleep(std::time::Duration::from_secs(2));

    TestContext {
        runtime_dir,
        pipewire,
    }
}

#[derive(Clone)]
struct Objects {
    map: Arc<RefCell<HashMap<Id, Box<dyn HasProxy>>>>,
}

unsafe impl Send for Objects {}

#[test]
fn test_lib() {
    let _test_context = start_pipewire();

    pipewire::init();

    let objects = Objects {
        map: Arc::new(RefCell::new(HashMap::new())),
    };

    let v = vec![("loop.name".to_string(), "pw-main-loop".to_string())];
    let main_loop = MainLoop::new(&Properties::new_vec(v)).unwrap();

    let context =
        Context::new(&main_loop, Properties::new()).expect("Context creation should not fail");

    let core = context.connect(None).unwrap();

    core.proxy().add_listener(ProxyEvents {
        destroy: some_closure!([^(objects)] {
            println!("core destroyed, clearing objects");
            objects.map.borrow_mut().clear();
        }),
        ..Default::default()
    });

    let mut timer_src = main_loop
        .add_timer(closure!([main_loop, core ^(objects)] _expirations, {
            assert_eq!(objects.map.borrow().len(), 1);
            core.disconnect();
            assert_eq!(objects.map.borrow().len(), 0);
            main_loop.quit();
        }))
        .unwrap();

    let timeout = libc::timespec {
        tv_sec: 2,
        tv_nsec: 0,
    };
    let res = main_loop.update_timer(&mut timer_src, &timeout, None, false);
    assert!(res.is_ok());

    let registry = core.registry().unwrap();

    registry.add_listener(RegistryEvents {
        global: some_closure!([registry ^(objects)] id, perms, type_, version, props, {
            println!("new global id {id}: {type_}/{version} ({perms}): {{ {props:?} }}");

            let object = match type_ {
                types::interface::CLIENT => {
                    let client = registry.bind(id, type_, version).unwrap();
                    let proxy = client.downcast_proxy::<Client>().unwrap();

                    proxy.add_listener(ProxyEvents {
                        removed: some_closure!([proxy ^(objects)] {
                            objects.map.borrow_mut().remove(&proxy.id());
                        }),
                        ..Default::default()
                    });

                    client
                }
                _ => return,
            };

            objects.map.borrow_mut().insert(id, object);
        }),
        global_remove: some_closure!([^(objects)] id, {
            println!("global {id} removed");
            let _ = objects.map.borrow_mut().remove(&id);
        }),
    });

    main_loop.run();

    assert_eq!(objects.map.borrow().len(), 0);
}
