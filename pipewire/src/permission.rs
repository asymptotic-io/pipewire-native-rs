// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use bitflags::bitflags;

use crate::Id;

bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PermissionBits : u32 {
        const R = (1 << 8);
        const READ = (1 << 8);
        const W = (1 << 7);
        const WRITE = (1 << 7);
        const X = (1 << 6);
        const EXECUTE = (1 << 6);
        const M = (1 << 3);
        const METADATA = (1 << 3);
        const L = (1 << 4);
        const LINK = (1 << 4);
    }
}

pub struct Permission {
    pub id: Id,
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
