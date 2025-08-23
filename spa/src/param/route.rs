// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros::EnumU32;

use crate::pod::types::ObjectType;

use super::ParamObject;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumU32)]
pub enum Route {
    Start,
    Index,
    Direction,
    Device,
    Name,
    Description,
    Priority,
    Available,
    Info,
    Profiles,
    Props,
    Devices,
    Profile,
    Save,
}

impl ParamObject for Route {
    const TYPE: ObjectType = ObjectType::ParamRoute;
}
