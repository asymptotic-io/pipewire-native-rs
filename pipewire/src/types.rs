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
    /// The Device interface
    pub const DEVICE: &str = "PipeWire:Interface:Device";
    /// The Factory interface
    pub const FACTORY: &str = "PipeWire:Interface:Factory";
    /// The Link interface
    pub const LINK: &str = "PipeWire:Interface:Link";
    /// The Metadata interface
    pub const METADATA: &str = "PipeWire:Interface:Metadata";
    /// The Module interface
    pub const MODULE: &str = "PipeWire:Interface:Module";
    /// The Node interface
    pub const NODE: &str = "PipeWire:Interface:Node";
    /// The Port interface
    pub const PORT: &str = "PipeWire:Interface:Port";
    /// The Registry interface
    pub const REGISTRY: &str = "PipeWire:Interface:Registry";
}

/// Types to deal with param objects
pub mod params {
    use pipewire_native_spa as spa;

    /// Because param objects are generic and depend on the context in which they are being used,
    /// we provide a construct a param object. The provided object and param types are used while
    /// creating the message sent to the server, and the `builder` callback is then called to let
    /// the caller set the required fields (which can be as complex as required).
    pub struct ParamBuilder {
        /// The object type for the param being built.
        pub object_type: spa::pod::types::ObjectType,
        /// The id of the param being built.
        pub param_id: spa::param::ParamType,
        /// A free form Pod builder for the individual object fields.
        pub builder:
            Box<dyn FnOnce(spa::pod::builder::ObjectBuilder) -> spa::pod::builder::ObjectBuilder>,
    }
}
