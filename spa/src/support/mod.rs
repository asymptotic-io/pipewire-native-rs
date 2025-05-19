// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::sync::LazyLock;

use plugin::Plugin;

pub mod ffi;
pub mod r#loop;
pub mod plugin;
pub mod system;

static PLUGIN: LazyLock<Plugin> = LazyLock::new(Plugin::new);

fn plugin() -> &'static Plugin {
    &PLUGIN
}
