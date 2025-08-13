// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tempfile;

use pipewire_native::{
    self as pipewire,
    context::Context,
    main_loop::MainLoop,
    properties::Properties,
    proxy::{client::Client, registry::RegistryEvents, HasProxy, ProxyEvents},
    some_closure, types, Refcounted,
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

#[test]
fn test_lib() {
    let _test_context = start_pipewire();

    pipewire::init();

    let objects = Rc::new(RefCell::new(HashMap::new()));

    let v = vec![("loop.name".to_string(), "pw-main-loop".to_string())];
    let main_loop = MainLoop::new(&Properties::new_vec(v)).unwrap();

    let context =
        Context::new(&main_loop, Properties::new()).expect("Context creation should not fail");

    let core = context.connect(None).unwrap();

    let objects_clone = objects.clone();
    core.proxy().add_listener(ProxyEvents {
        destroy: some_closure!(_core <- core, {
            println!("core destroyed, clearing objects");
            objects_clone.borrow_mut().clear();
        }),
        ..Default::default()
    });

    let ml = context.main_loop();

    let ml_clone = ml.clone();
    let core_clone = core.clone();
    let objects_clone = objects.clone();
    let mut timer_src = ml
        .add_timer(Box::new(move |_expirations| {
            assert_eq!(objects_clone.borrow().len(), 1);
            core_clone.disconnect();
            assert_eq!(objects_clone.borrow().len(), 0);
            ml_clone.quit();
        }))
        .unwrap();

    let timeout = libc::timespec {
        tv_sec: 2,
        tv_nsec: 0,
    };
    let res = ml.update_timer(&mut timer_src, &timeout, None, false);
    assert!(res.is_ok());

    let registry = core.registry().unwrap();
    let objects_clone_g = objects.clone();
    let objects_clone_gr = objects.clone();

    registry.add_listener(RegistryEvents {
        global: some_closure!(registry, id, perms, type_, version, props, {
            println!("new global id {id}: {type_}/{version} ({perms}): {{ {props:?} }}");

            let objects_clone_pr = objects_clone_g.clone();

            let object = match type_ {
                types::interface::CLIENT => {
                    let client = registry.bind(id, type_, version).unwrap();
                    let proxy = client.downcast_proxy::<Client>().unwrap();

                    proxy.add_listener(ProxyEvents {
                        removed: some_closure!(proxy, {
                            objects_clone_pr.borrow_mut().remove(&proxy.id());
                        }),
                        ..Default::default()
                    });

                    client
                }
                _ => return,
            };

            objects_clone_g.borrow_mut().insert(id, object);
        }),
        global_remove: some_closure!(_registry <- registry, id, {
            println!("global {id} removed");
            let _ = objects_clone_gr.borrow_mut().remove(&id);
        }),
    });

    ml.run();

    assert_eq!(objects.borrow().len(), 0);
}
