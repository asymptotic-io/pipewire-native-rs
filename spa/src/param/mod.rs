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
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, EnumU32)]
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

impl ParamType {
    pub fn object_type(&self) -> Option<ObjectType> {
        match self {
            Self::Invalid => None,
            Self::PropInfo => Some(ObjectType::PropInfo),
            Self::Props => Some(ObjectType::Props),
            Self::EnumFormat => Some(ObjectType::Format),
            Self::Format => Some(ObjectType::Format),
            Self::Buffers => Some(ObjectType::ParamBuffers),
            Self::Meta => Some(ObjectType::ParamMeta),
            Self::IO => Some(ObjectType::ParamIo),
            Self::EnumProfile => Some(ObjectType::ParamProfile),
            Self::Profile => Some(ObjectType::ParamProfile),
            Self::EnumPortConfig => Some(ObjectType::ParamPortConfig),
            Self::PortConfig => Some(ObjectType::ParamPortConfig),
            Self::EnumRoute => Some(ObjectType::ParamRoute),
            Self::Route => Some(ObjectType::ParamRoute),
            Self::Control => None, // Sequence
            Self::Latency => Some(ObjectType::ParamLatency),
            Self::ProcessLatency => Some(ObjectType::ParamProcessLatency),
            Self::Tag => Some(ObjectType::ParamTag),
        }
    }

    pub fn key_to_string(&self, id: u32) -> Option<String> {
        match self {
            Self::Invalid => None,
            Self::PropInfo => props::PropInfo::try_from(id).ok().map(|p| format!("{p:?}")),
            Self::Props => props::Prop::try_from(id).ok().map(|p| format!("{p:?}")),
            Self::EnumFormat => format::Format::try_from(id).ok().map(|p| format!("{p:?}")),
            Self::Format => format::Format::try_from(id).ok().map(|p| format!("{p:?}")),
            Self::Buffers => buffers::Buffers::try_from(id)
                .ok()
                .map(|p| format!("{p:?}")),
            Self::Meta => buffers::Meta::try_from(id).ok().map(|p| format!("{p:?}")),
            Self::IO => None, // TODO
            Self::EnumProfile => profile::Profile::try_from(id)
                .ok()
                .map(|p| format!("{p:?}")),
            Self::Profile => profile::Profile::try_from(id)
                .ok()
                .map(|p| format!("{p:?}")),
            Self::EnumPortConfig => None, // TODO
            Self::PortConfig => None,     // TODO
            Self::EnumRoute => route::Route::try_from(id).ok().map(|p| format!("{p:?}")),
            Self::Route => route::Route::try_from(id).ok().map(|p| format!("{p:?}")),
            Self::Control => None,        // Sequence
            Self::Latency => None,        // TODO
            Self::ProcessLatency => None, // TODO
            Self::Tag => None,            // TODO
        }
    }
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

pub trait ParamObject: std::fmt::Debug {
    const TYPE: ObjectType;
}
