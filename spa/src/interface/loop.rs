// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use super::plugin::Interface;
use crate::{
    flags,
    interface::ffi::{CControlHooks, CHook},
};
use std::{any::Any, os::fd::RawFd, pin::Pin, time::Duration};

#[derive(Copy, Clone, Debug)]
pub struct Source {
    pub fd: RawFd,
    pub mask: u32,
    pub rmask: u32,
}

pub type SourceFn = dyn FnMut(&Source) + 'static;
pub type InvokeFn = dyn FnMut(bool, u32, &[u8]) -> i32 + 'static;

pub struct LoopImpl {
    pub inner: Pin<Box<dyn Any>>,

    pub add_source: fn(&mut LoopImpl, source: &Source, func: Box<SourceFn>) -> std::io::Result<i32>,
    pub update_source: fn(&mut LoopImpl, source: &Source) -> std::io::Result<i32>,
    pub remove_source: fn(&mut LoopImpl, fd: RawFd) -> std::io::Result<i32>,
    #[allow(clippy::type_complexity)]
    pub invoke: fn(
        this: &mut LoopImpl,
        seq: u32,
        data: &[u8],
        block: bool,
        func: Box<InvokeFn>,
    ) -> std::io::Result<i32>,
}

impl LoopImpl {
    pub fn add_source(&mut self, source: &Source, func: Box<SourceFn>) -> std::io::Result<i32> {
        (self.add_source)(self, source, func)
    }

    pub fn update_source(&mut self, source: &Source) -> std::io::Result<i32> {
        (self.update_source)(self, source)
    }

    pub fn remove_source(&mut self, fd: RawFd) -> std::io::Result<i32> {
        (self.remove_source)(self, fd)
    }

    pub fn invoke(
        &mut self,
        seq: u32,
        data: &[u8],
        block: bool,
        func: Box<InvokeFn>,
    ) -> std::io::Result<i32> {
        (self.invoke)(self, seq, data, block, func)
    }
}

impl Interface for LoopImpl {
    unsafe fn make_native(&self) -> *mut super::ffi::CInterface {
        crate::support::ffi::r#loop::make_native(self)
    }

    unsafe fn free_native(loop_: *mut super::ffi::CInterface) {
        crate::support::ffi::r#loop::free_native(loop_)
    }
}

pub struct LoopControlMethodsImpl {
    pub inner: Pin<Box<dyn Any>>,

    pub get_fd: fn(&LoopControlMethodsImpl) -> u32,
    pub add_hook: fn(&LoopControlMethodsImpl, hook: &CHook, hooks: &CControlHooks, data: u64),
    pub enter: fn(&LoopControlMethodsImpl),
    pub leave: fn(&LoopControlMethodsImpl),
    pub iterate: fn(&LoopControlMethodsImpl, timeout: Option<Duration>) -> std::io::Result<i32>,
    pub check: fn(&LoopControlMethodsImpl) -> std::io::Result<i32>,
    pub lock: fn(&LoopControlMethodsImpl) -> std::io::Result<i32>,
    pub unlock: fn(&LoopControlMethodsImpl) -> std::io::Result<i32>,
    pub get_time: fn(&LoopControlMethodsImpl, timeout: Duration) -> std::io::Result<libc::timespec>,
    pub wait: fn(&LoopControlMethodsImpl, abstime: &libc::timespec) -> std::io::Result<i32>,
    pub signal: fn(&LoopControlMethodsImpl, wait_for_accept: bool) -> std::io::Result<i32>,
    pub accept: fn(&LoopControlMethodsImpl) -> std::io::Result<i32>,
}

impl LoopControlMethodsImpl {
    pub fn get_fd(&self) -> u32 {
        (self.get_fd)(self)
    }

    pub fn add_hook(&self, hook: &CHook, hooks: &CControlHooks, data: u64) {
        (self.add_hook)(self, hook, hooks, data)
    }

    pub fn enter(&self) {
        (self.enter)(self)
    }

    pub fn leave(&self) {
        (self.leave)(self)
    }

    pub fn iterate(&self, timeout: Option<Duration>) -> std::io::Result<i32> {
        (self.iterate)(self, timeout)
    }

    pub fn check(&self) -> std::io::Result<i32> {
        (self.check)(self)
    }

    pub fn lock(&self) -> std::io::Result<i32> {
        (self.lock)(self)
    }

    pub fn unlock(&self) -> std::io::Result<i32> {
        (self.unlock)(self)
    }

    pub fn get_time(&self, timeout: Duration) -> std::io::Result<libc::timespec> {
        (self.get_time)(self, timeout)
    }

    pub fn wait(&self, abstime: &libc::timespec) -> std::io::Result<i32> {
        (self.wait)(self, abstime)
    }

    pub fn signal(&self, wait_for_accept: bool) -> std::io::Result<i32> {
        (self.signal)(self, wait_for_accept)
    }

    pub fn accept(&self) -> std::io::Result<i32> {
        (self.accept)(self)
    }
}

impl Interface for LoopControlMethodsImpl {
    unsafe fn make_native(&self) -> *mut super::ffi::CInterface {
        crate::support::ffi::r#loop::control::make_native(self)
    }

    unsafe fn free_native(loop_: *mut super::ffi::CInterface) {
        crate::support::ffi::r#loop::control::free_native(loop_)
    }
}

pub type SourceIoFn = dyn FnMut(RawFd, u32) + 'static;
pub type SourceIdleFn = dyn FnMut() + 'static;
pub type SourceEventFn = dyn FnMut(u64) + 'static;
pub type SourceTimerFn = dyn FnMut(u64) + 'static;
pub type SourceSignalFn = dyn FnMut(i32) + 'static;

pub enum LoopUtilsSourceCb {
    Io(Box<SourceIoFn>),
    Idle(Box<SourceIdleFn>),
    Event(Box<SourceEventFn>),
    Timer(Box<SourceTimerFn>),
    Signal(Box<SourceSignalFn>),
}

pub struct LoopUtilsSource {
    pub cb: LoopUtilsSourceCb,
    pub inner: Box<dyn Any>,
}

#[allow(clippy::type_complexity)]
pub struct LoopUtilsImpl {
    pub inner: Pin<Box<dyn Any>>,

    pub add_io: fn(
        &LoopUtilsImpl,
        fd: RawFd,
        mask: flags::Io,
        close: bool,
        func: Box<SourceIoFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>>,
    pub update_io: fn(
        &LoopUtilsImpl,
        source: &mut Pin<Box<LoopUtilsSource>>,
        mask: flags::Io,
    ) -> std::io::Result<i32>,
    pub add_idle: fn(
        &LoopUtilsImpl,
        enabled: bool,
        func: Box<SourceIdleFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>>,
    pub enable_idle: fn(
        &LoopUtilsImpl,
        source: &mut Pin<Box<LoopUtilsSource>>,
        enabled: bool,
    ) -> std::io::Result<i32>,
    pub add_event:
        fn(&LoopUtilsImpl, func: Box<SourceEventFn>) -> Option<Pin<Box<LoopUtilsSource>>>,
    pub signal_event:
        fn(&LoopUtilsImpl, source: &mut Pin<Box<LoopUtilsSource>>) -> std::io::Result<i32>,
    pub add_timer:
        fn(&LoopUtilsImpl, func: Box<SourceTimerFn>) -> Option<Pin<Box<LoopUtilsSource>>>,
    pub update_timer: fn(
        &LoopUtilsImpl,
        source: &mut Pin<Box<LoopUtilsSource>>,
        value: &libc::timespec,
        interval: Option<&libc::timespec>,
        absolute: bool,
    ) -> std::io::Result<i32>,
    pub add_signal: fn(
        &LoopUtilsImpl,
        signal_number: i32,
        func: Box<SourceSignalFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>>,
    pub destroy_source: fn(&LoopUtilsImpl, source: Pin<Box<LoopUtilsSource>>),
}

impl LoopUtilsImpl {
    pub fn add_io(
        &self,
        fd: RawFd,
        mask: flags::Io,
        close: bool,
        func: Box<SourceIoFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        (self.add_io)(self, fd, mask, close, func)
    }

    pub fn update_io(
        &self,
        source: &mut Pin<Box<LoopUtilsSource>>,
        mask: flags::Io,
    ) -> std::io::Result<i32> {
        (self.update_io)(self, source, mask)
    }

    pub fn add_idle(
        &self,
        enabled: bool,
        func: Box<SourceIdleFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        (self.add_idle)(self, enabled, func)
    }

    pub fn enable_idle(
        &self,
        source: &mut Pin<Box<LoopUtilsSource>>,
        enabled: bool,
    ) -> std::io::Result<i32> {
        (self.enable_idle)(self, source, enabled)
    }

    pub fn add_event(&self, func: Box<SourceEventFn>) -> Option<Pin<Box<LoopUtilsSource>>> {
        (self.add_event)(self, func)
    }

    pub fn signal_event(&self, source: &mut Pin<Box<LoopUtilsSource>>) -> std::io::Result<i32> {
        (self.signal_event)(self, source)
    }

    pub fn add_timer(&self, func: Box<SourceTimerFn>) -> Option<Pin<Box<LoopUtilsSource>>> {
        (self.add_timer)(self, func)
    }

    pub fn update_timer(
        &self,
        source: &mut Pin<Box<LoopUtilsSource>>,
        value: &libc::timespec,
        interval: Option<&libc::timespec>,
        absolute: bool,
    ) -> std::io::Result<i32> {
        (self.update_timer)(self, source, value, interval, absolute)
    }

    pub fn add_signal(
        &self,
        signal_number: i32,
        func: Box<SourceSignalFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        (self.add_signal)(self, signal_number, func)
    }

    pub fn destroy_source(&self, source: Pin<Box<LoopUtilsSource>>) {
        (self.destroy_source)(self, source)
    }
}

impl Interface for LoopUtilsImpl {
    unsafe fn make_native(&self) -> *mut super::ffi::CInterface {
        crate::support::ffi::r#loop::utils::make_native(self)
    }

    unsafe fn free_native(loop_: *mut super::ffi::CInterface) {
        crate::support::ffi::r#loop::utils::free_native(loop_)
    }
}
