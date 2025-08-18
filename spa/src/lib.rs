// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

pub mod dict;
pub mod flags;
pub mod hook;
pub mod interface;
pub mod param;
pub mod pod;
pub mod support;

pub fn atob(s: &str) -> bool {
    s == "true" || s == "1"
}
