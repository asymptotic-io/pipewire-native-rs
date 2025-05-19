// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    ffi::{c_void, CStr, CString},
    pin::Pin,
};

use cpu::CpuImpl;
use ffi::{CInterface, CSupport};
use log::LogImpl;
use r#loop::LoopImpl;
use system::SystemImpl;

use crate::support;

pub mod cpu;
pub mod ffi;
pub mod log;
pub mod r#loop;
pub mod plugin;
pub mod system;

/* Well-known interface names */
pub const LOG: &str = "Spa:Pointer:Interface:Log";
pub const LOOP: &str = "Spa:Pointer:Interface:Loop";
pub const SYSTEM: &str = "Spa:Pointer:Interface:System";
pub const CPU: &str = "Spa:Pointer:Interface:Cpu";

pub struct Support {
    cpu: Option<Pin<Box<CpuImpl>>>,
    log: Option<Pin<Box<LogImpl>>>,
    system: Option<Pin<Box<SystemImpl>>>,
    loop_: Option<Pin<Box<LoopImpl>>>,
    /* We keep a C-compatible array that won't get moved around, so we can reliably pass it on to
     * plugins */
    all: Vec<CSupport>,
}

impl Default for Support {
    fn default() -> Self {
        Support {
            cpu: None,
            log: None,
            system: None,
            loop_: None,
            /* Reserve enough space so the array is always valid */
            all: Vec::with_capacity(16),
        }
    }
}

impl Support {
    pub fn new() -> Support {
        Support::default()
    }

    pub fn all(&self) -> &Vec<CSupport> {
        &self.all
    }

    fn add_or_update(&mut self, name: &str, data: *mut CInterface) {
        for s in self.all.iter_mut() {
            let type_ = unsafe { CStr::from_ptr(s.type_).to_str() };
            if type_ == Ok(name) {
                s.data = data as *mut c_void;
                return;
            }
        }

        self.all.push(CSupport {
            type_: support::ffi::c_string(name).into_raw(),
            data: data as *mut c_void,
        });
    }

    pub fn set_log(&mut self, log: Box<LogImpl>) {
        self.log = Some(Pin::new(log));
        self.add_or_update(
            LOG,
            support::ffi::log::make_native(self.log.as_ref().unwrap()),
        );
    }

    pub fn set_system(&mut self, system: Box<SystemImpl>) {
        self.system = Some(Pin::new(system));
        self.add_or_update(
            SYSTEM,
            support::ffi::system::make_native(self.system.as_ref().unwrap()),
        );
    }

    pub fn set_loop(&mut self, loop_: Box<LoopImpl>) {
        self.loop_ = Some(Pin::new(loop_));
        self.add_or_update(
            LOOP,
            support::ffi::r#loop::make_native(self.loop_.as_ref().unwrap()),
        );
    }

    pub fn set_cpu(&mut self, cpu: Box<CpuImpl>) {
        self.cpu = Some(Pin::new(cpu));
        self.add_or_update(
            CPU,
            support::ffi::cpu::make_native(self.cpu.as_ref().unwrap()),
        );
    }
}

impl Drop for Support {
    fn drop(&mut self) {
        for s in self.all.iter_mut() {
            let type_ = unsafe { CString::from_raw(s.type_) };
            match type_.to_str().unwrap() {
                CPU => support::ffi::cpu::free_native(s.data as *mut CInterface),
                LOG => support::ffi::log::free_native(s.data as *mut CInterface),
                SYSTEM => support::ffi::system::free_native(s.data as *mut CInterface),
                LOOP => support::ffi::r#loop::free_native(s.data as *mut CInterface),
                _ => unreachable!(),
            }
        }
    }
}
