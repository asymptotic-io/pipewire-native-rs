// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::sync::Arc;

use pipewire_native::{
    self as pipewire, context::Context, main_loop::MainLoop, properties::Properties,
};
use pipewire_native_spa::dict::Dict;

#[test]
fn test_lib() {
    pipewire::init();

    let v: Vec<(String, String)> = vec![("loop.name".to_string(), "pw-main-loop".to_string())];
    let ml = MainLoop::new(&Dict::new(v)).unwrap();

    let context =
        Context::new(Arc::new(ml), Properties::new()).expect("Context creation should not fail");

    let core = context.connect(None).unwrap();

    let ml = context.main_loop();

    let ml2 = ml.clone();
    let mut timer_src = ml
        .add_timer(Box::new(move |_expirations| {
            ml2.quit();
        }))
        .unwrap();

    let timeout = libc::timespec {
        tv_sec: 5,
        tv_nsec: 0,
    };
    let res = ml.update_timer(&mut timer_src, &timeout, None, true);
    assert!(res.is_ok());

    ml.run();
}
