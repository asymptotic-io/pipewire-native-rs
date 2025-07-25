// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

pub type ObjectType = &'static str;

pub mod interface {
    pub const CLIENT: &str = "PipeWire:Interface:Client";
    pub const CORE: &str = "PipeWire:Interface:Core";
}
