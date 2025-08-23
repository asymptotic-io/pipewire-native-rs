// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use bitflags::bitflags;
use pipewire_native_macros::EnumU32;

use crate::pod::types::ObjectType;

pub mod buffers;
pub mod format;
pub mod profile;
pub mod props;
pub mod route;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumU32)]
pub enum ParamType {
    Invalid,
    PropInfo,
    Props,
    EnumFormat,
    Format,
    Buffers,
    Meta,
    IO,
    EnumProfile,
    Profile,
    EnumPortConfig,
    PortConfig,
    EnumRoute,
    Route,
    Control,
    Latency,
    ProcessLatency,
    Tag,
}

bitflags! {
    #[repr(C)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ParamInfoFlags : u32 {
        const SERIAL    = 1 << 0;
        const READ      = 1 << 1;
        const WRITE     = 1 << 2;
        const READWRITE = (1 << 1) | (1 << 2);
    }
}

pub trait ParamObject {
    const TYPE: ObjectType;
}
