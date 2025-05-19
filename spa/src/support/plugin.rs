// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use crate::{
    dict::Dict,
    interface::{
        self,
        plugin::{Handle, HandleFactory, Interface, InterfaceInfo},
    },
};

use super::system;

pub struct Plugin {}

pub struct PluginHandle {}

impl Default for Plugin {
    fn default() -> Self {
        Self::new()
    }
}

impl Plugin {
    pub fn new() -> Self {
        Self {}
    }
}

impl HandleFactory for Plugin {
    fn version(&self) -> u32 {
        0
    }

    fn name(&self) -> &str {
        "rust-support"
    }

    fn info(&self) -> Option<&Dict> {
        None
    }

    fn init(
        &self,
        _: Option<Dict>,
        _: &interface::Support,
    ) -> std::io::Result<impl Handle + 'static> {
        Ok(PluginHandle {})
    }

    fn enum_interface_info(&self) -> Vec<InterfaceInfo> {
        vec![InterfaceInfo {
            type_: interface::SYSTEM.to_string(),
        }]
    }
}

impl Handle for PluginHandle {
    fn version(&self) -> u32 {
        0
    }

    fn get_interface(&self, type_: &str) -> Option<Box<dyn Interface>> {
        match type_ {
            interface::SYSTEM => Some(Box::new(system::new())),
            _ => None,
        }
    }
}
