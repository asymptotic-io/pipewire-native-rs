// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

fn main() {
    cc::Build::new()
        .file("src/support/ffi/log.c")
        .file("src/support/ffi/system.c")
        .compile("support-ffi");

    println!("cargo::rerun-if-changed=src/support/ffi/plugin.h");
    println!("cargo::rerun-if-changed=src/support/ffi/log.c");
    println!("cargo::rerun-if-changed=src/support/ffi/system.c");
}
