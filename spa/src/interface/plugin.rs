// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::any::Any;

use crate::dict::Dict;

pub const LOG_FACTORY: &str = "support.log";
pub const SYSTEM_FACTORY: &str = "support.system";
pub const CPU_FACTORY: &str = "support.cpu";

pub trait Interface: Any {}

pub struct InterfaceInfo {
    pub type_: String,
}

pub trait HandleFactory {
    /* Data fields */
    fn version(&self) -> u32;
    fn name(&self) -> &str;
    fn info(&self) -> Option<&Dict>;

    /* Methods */
    /* The 'static trait bound allows the borrow-checker to not assume that the output type holds a
     * reference to `support` which is a bit of a lie, but we're expressing an implied contract
     * that support will be a global, and that the associated support handles that are being
     * referenced inside the `Support` structure will be dropped at the same time as the structure
     * by the loading entity. */
    fn init(
        &self,
        info: Option<Dict>,
        support: &super::Support,
    ) -> std::io::Result<impl Handle + 'static>;
    fn enum_interface_info(&self) -> Vec<InterfaceInfo>;
}

pub trait Handle {
    /* Data fields */
    fn version(&self) -> u32;

    /* Methods */
    fn get_interface(&self, type_: &str) -> Option<Box<dyn Interface>>;
}
