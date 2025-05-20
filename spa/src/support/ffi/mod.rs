// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::ffi::CString;

pub mod cpu;
pub mod log;
pub mod r#loop;
pub mod plugin;
pub mod system;

/* TODO: can we avoid an allocation here? */
pub fn c_string(s: &str) -> CString {
    CString::new(s).expect("&str should wrap as CString")
}
