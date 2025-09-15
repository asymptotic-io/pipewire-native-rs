// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use bitflags::bitflags;

use crate::Id;

bitflags! {
    /// Represents permissions within PipeWire.
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PermissionBits : u32 {
        /// Object can be read and events received.
        const READ = (1 << 8);
        /// Alias for [`Self::READ`]
        const R = (1 << 8);
        /// Methods that modify the object can be called.
        const WRITE = (1 << 7);
        /// Alias for [`Self::WRITE`]
        const W = (1 << 7);
        /// Methods can be called on the object.
        const EXECUTE = (1 << 6);
        /// Alias for [`Self::EXECUTE`]
        const X = (1 << 6);
        /// Metadata can be set on the object.
        const METADATA = (1 << 3);
        /// Alias for [`Self::METADATA`]
        const M = (1 << 3);
        /// A link can be made between a node that doesn't have permission to see another node.
        const LINK = (1 << 4);
        /// Alias for [`Self::LINK`]
        const L = (1 << 4);
    }
}

/// Represents permissions on an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permission {
    /// The object on which the permissions apply.
    pub id: Id,
    /// The permissions of the object.
    pub permissions: PermissionBits,
}

impl std::fmt::Display for PermissionBits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.contains(PermissionBits::READ) {
            write!(f, "r")
        } else {
            write!(f, "-")
        }?;
        if self.contains(PermissionBits::WRITE) {
            write!(f, "w")
        } else {
            write!(f, "-")
        }?;
        if self.contains(PermissionBits::EXECUTE) {
            write!(f, "x")
        } else {
            write!(f, "-")
        }?;
        if self.contains(PermissionBits::METADATA) {
            write!(f, "m")
        } else {
            write!(f, "-")
        }?;
        if self.contains(PermissionBits::LINK) {
            write!(f, "l")
        } else {
            write!(f, "-")
        }
    }
}
