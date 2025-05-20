// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Sanchayan Maity

use std::{any::Any, pin::Pin};

use super::plugin::Interface;

pub struct CpuImpl {
    pub inner: Pin<Box<dyn Any>>,

    pub get_flags: fn(this: &CpuImpl) -> u32,
    pub force_flags: fn(this: &CpuImpl, flags: u32) -> i32,
    pub get_count: fn(this: &CpuImpl) -> u32,
    pub get_max_align: fn(this: &CpuImpl) -> u32,
    pub get_vm_type: fn(this: &CpuImpl) -> u32,
    pub zero_denormals: fn(this: &CpuImpl, enable: bool) -> i32,
}

impl CpuImpl {
    pub fn get_flags(&self) -> u32 {
        (self.get_flags)(self)
    }

    pub fn force_flags(&self, flags: u32) -> i32 {
        (self.force_flags)(self, flags)
    }

    pub fn get_count(&self) -> u32 {
        (self.get_count)(self)
    }

    pub fn get_max_align(&self) -> u32 {
        (self.get_max_align)(self)
    }

    pub fn get_vm_type(&self) -> u32 {
        (self.get_vm_type)(self)
    }

    pub fn zero_denormals(&self, enable: bool) -> i32 {
        (self.zero_denormals)(self, enable)
    }
}

impl Interface for CpuImpl {}

unsafe impl Send for CpuImpl {}
unsafe impl Sync for CpuImpl {}
