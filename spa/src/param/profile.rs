// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros::EnumU32;

use crate::pod::types::ObjectType;

use super::ParamObject;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumU32)]
pub enum Profile {
    Start,
    Index,
    Name,
    Description,
    Priority,
    Available,
    Info,
    Classes,
    Save,
}

impl ParamObject for Profile {
    const TYPE: ObjectType = ObjectType::ParamProfile;
}
