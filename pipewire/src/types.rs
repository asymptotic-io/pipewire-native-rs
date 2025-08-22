// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

/// An type description of a PipeWire interface.
pub type ObjectType = &'static str;

/// Names of PipeWire interfaces.
pub mod interface {
    /// The Client interface.
    pub const CLIENT: &str = "PipeWire:Interface:Client";
    /// The Core interface
    pub const CORE: &str = "PipeWire:Interface:Core";
    /// The Module interface
    pub const MODULE: &str = "PipeWire:Interface:Module";
    /// The Registry interface
    pub const REGISTRY: &str = "PipeWire:Interface:Registry";
}
