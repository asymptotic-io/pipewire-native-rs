// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

fn main() {
    match pkg_config::get_variable("libspa-0.2", "plugindir") {
        Ok(plugindir) => println!("cargo::rustc-env=SPA_DEFAULT_PLUGINDIR={plugindir}"),
        Err(e) => {
            println!("cargo::warning=pkg-config error: {e})");
            println!("cargo::warning=Could not find SPA plugindir from pkg-config, using default");
            println!("cargo::rustc-env=SPA_DEFAULT_PLUGINDIR=/usr/lib64/spa-0.2");
        }
    }
}
