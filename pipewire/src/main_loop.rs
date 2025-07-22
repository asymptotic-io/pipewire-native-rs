// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Sanchayan Maity

use pipewire_native_spa as spa;
use spa::dict::Dict;
use spa::flags;
use spa::interface::ffi::{CControlHooks, CHook};
use spa::interface::r#loop::{
    LoopUtilsSource, SourceEventFn, SourceIdleFn, SourceIoFn, SourceSignalFn, SourceTimerFn,
};
use spa::{emit_hook, hook::HookList};

use std::os::fd::RawFd;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use crate::support::LoopSupport;
use crate::GLOBAL_SUPPORT;
use crate::{debug, default_topic, log, trace};

default_topic!(log::topic::MAIN_LOOP);

pub struct MainLoopEvents {
    destroy: Box<dyn FnMut()>,
}

impl MainLoopEvents {
    pub fn new(destroy_cb: Box<dyn FnMut()>) -> Self {
        Self {
            destroy: destroy_cb,
        }
    }
}

unsafe impl Send for MainLoopEvents {}
unsafe impl Sync for MainLoopEvents {}

#[derive(Clone)]
pub struct MainLoop {
    inner: Arc<InnerMainLoop>,
}

impl MainLoop {
    pub fn new(props: &Dict) -> Option<MainLoop> {
        let Some(l) = InnerMainLoop::new(props) else {
            return None;
        };

        debug!("Creating main loop");

        Some(MainLoop { inner: Arc::new(l) })
    }

    pub fn run(&self) {
        debug!("run");
        InnerMainLoop::run(&self.inner);
    }

    pub fn quit(&self) {
        debug!("quit");
        InnerMainLoop::quit(&self.inner);
    }

    pub fn add_listener(&self, events: MainLoopEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    // Loop control methods
    pub fn get_fd(&self) -> u32 {
        self.inner.support.loop_control.get_fd()
    }

    pub fn add_hook(&self, hook: &CHook, hooks: &CControlHooks, data: u64) {
        self.inner.support.loop_control.add_hook(hook, hooks, data)
    }

    pub fn enter(&self) {
        trace!("enter");
        self.inner.support.loop_control.enter()
    }

    pub fn leave(&self) {
        trace!("leave");
        self.inner.support.loop_control.leave()
    }

    pub fn iterate(&self, timeout: Option<std::time::Duration>) -> std::io::Result<i32> {
        trace!("iterate");
        self.inner.support.loop_control.iterate(timeout)
    }

    pub fn check(&self) -> std::io::Result<i32> {
        self.inner.support.loop_control.check()
    }

    pub fn lock(&self) -> std::io::Result<i32> {
        trace!("lock");
        self.inner.support.loop_control.lock()
    }

    pub fn unlock(&self) -> std::io::Result<i32> {
        trace!("unlock");
        self.inner.support.loop_control.unlock()
    }

    pub fn get_time(&self, timeout: std::time::Duration) -> std::io::Result<libc::timespec> {
        self.inner.support.loop_control.get_time(timeout)
    }

    pub fn wait(&self, abstime: &libc::timespec) -> std::io::Result<i32> {
        debug!("wait");
        self.inner.support.loop_control.wait(abstime)
    }

    pub fn signal(&self, wait_for_accept: bool) -> std::io::Result<i32> {
        debug!("signal");
        self.inner.support.loop_control.signal(wait_for_accept)
    }

    pub fn accept(&self) -> std::io::Result<i32> {
        debug!("accept");
        self.inner.support.loop_control.accept()
    }

    // Loop utils
    pub fn add_io(
        &self,
        fd: RawFd,
        mask: flags::Io,
        close: bool,
        func: Box<SourceIoFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        self.inner.support.loop_utils.add_io(fd, mask, close, func)
    }

    pub fn update_io(
        &self,
        source: &mut Pin<Box<LoopUtilsSource>>,
        mask: flags::Io,
    ) -> std::io::Result<i32> {
        self.inner.support.loop_utils.update_io(source, mask)
    }

    pub fn add_idle(
        &self,
        enabled: bool,
        func: Box<SourceIdleFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        self.inner.support.loop_utils.add_idle(enabled, func)
    }

    pub fn enable_idle(
        &self,
        source: &mut Pin<Box<LoopUtilsSource>>,
        enabled: bool,
    ) -> std::io::Result<i32> {
        debug!("idle {enabled}");
        self.inner.support.loop_utils.enable_idle(source, enabled)
    }

    pub fn add_event(&self, func: Box<SourceEventFn>) -> Option<Pin<Box<LoopUtilsSource>>> {
        self.inner.support.loop_utils.add_event(func)
    }

    pub fn signal_event(&self, source: &mut Pin<Box<LoopUtilsSource>>) -> std::io::Result<i32> {
        self.inner.support.loop_utils.signal_event(source)
    }

    pub fn add_timer(&self, func: Box<SourceTimerFn>) -> Option<Pin<Box<LoopUtilsSource>>> {
        self.inner.support.loop_utils.add_timer(func)
    }

    pub fn update_timer(
        &self,
        source: &mut Pin<Box<LoopUtilsSource>>,
        value: &libc::timespec,
        interval: Option<&libc::timespec>,
        absolute: bool,
    ) -> std::io::Result<i32> {
        self.inner
            .support
            .loop_utils
            .update_timer(source, value, interval, absolute)
    }

    pub fn add_signal(
        &self,
        signal_number: i32,
        func: Box<SourceSignalFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        self.inner
            .support
            .loop_utils
            .add_signal(signal_number, func)
    }

    pub fn destroy_source(&self, source: Pin<Box<LoopUtilsSource>>) {
        self.inner.support.loop_utils.destroy_source(source)
    }

    pub fn set_name(&mut self, name: &str) {
        debug!("main loop name {name}");
        if let Some(inner) = Arc::get_mut(&mut self.inner) {
            inner.name = name.to_string()
        }
    }

    pub(crate) fn get_support(&self) -> LoopSupport {
        self.inner.support.clone()
    }
}

struct InnerMainLoop {
    support: LoopSupport,
    // This is an atomic because it is hard to convince Rust that this will only be mutated on one
    // thread (i.e. the one on which run() is called
    running: AtomicBool,
    name: String,
    hooks: Arc<Mutex<HookList<MainLoopEvents>>>,
}

impl Drop for InnerMainLoop {
    fn drop(&mut self) {
        self.destroy();
    }
}

impl InnerMainLoop {
    pub fn new(props: &Dict) -> Option<InnerMainLoop> {
        let support = GLOBAL_SUPPORT
            .get()
            .expect("Global support should be initialised");

        let name = if let Some(n) = props.lookup("loop.name") {
            n.to_string()
        } else {
            "main.loop".to_string()
        };

        Some(InnerMainLoop {
            support: support.loop_().clone(),
            running: AtomicBool::new(false),
            name,
            hooks: HookList::new(),
        })
    }

    fn destroy(&self) {
        emit_hook!(self.hooks, destroy,);
    }

    fn run(this: &Arc<Self>) {
        if this
            .running
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return;
        }

        this.support.loop_control.enter();

        while this.running.load(Ordering::Relaxed) {
            if let Err(res) = this
                .support
                .loop_control
                .iterate(Some(std::time::Duration::MAX))
            {
                if res.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
            }
        }

        this.support.loop_control.leave();
    }

    fn quit(this: &Arc<Self>) {
        let this_ = this.clone();

        let stop = move |_block: bool, _seq: u32, _data: &[u8]| {
            this_.running.store(false, Ordering::Relaxed);
            0
        };

        let _ = this.support.loop_.invoke(1, &[], false, Box::new(stop));
    }
}
