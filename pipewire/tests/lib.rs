// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::sync::Arc;

use pipewire_native::{self as pipewire, main_loop::MainLoop, properties::Properties};
use pipewire_native_spa::dict::Dict;

#[test]
fn test_lib() {
    pipewire::init();

    let v: Vec<(String, String)> = vec![("loop.name".to_string(), "pw-main-loop".to_string())];
    let ml = MainLoop::new(&Dict::new(v)).unwrap();

    let context = pipewire::context::Context::new(Arc::new(ml), Properties::new());

    assert!(context.is_ok());
}
