// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Sanchayan Maity

use pipewire_native_spa as spa;

use spa::flags;
use spa::interface::r#loop::LoopUtilsSource;
use spa::{emit_hook, hook::HookList};

use std::os::fd::RawFd;
use std::pin::Pin;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use crate::GLOBAL_SUPPORT;
use crate::{debug, default_topic, log, new_refcounted, properties::Properties, refcounted, trace};

default_topic!(log::topic::MAIN_LOOP);

pub struct MainLoopEvents {
    destroy: Option<Box<dyn FnMut()>>,
}

impl MainLoopEvents {
    pub fn new(destroy_cb: Box<dyn FnMut()>) -> Self {
        Self {
            destroy: Some(destroy_cb),
        }
    }
}

unsafe impl Send for MainLoopEvents {}
unsafe impl Sync for MainLoopEvents {}

#[derive(Clone)]
pub(crate) struct LoopSupport {
    #[allow(dead_code)]
    handle: Arc<Box<dyn spa::interface::plugin::Handle + Send + Sync>>,
    pub(crate) loop_: Arc<Pin<Box<spa::interface::r#loop::LoopImpl>>>,
    pub(crate) loop_utils: Arc<Pin<Box<spa::interface::r#loop::LoopUtilsImpl>>>,
    pub(crate) loop_control: Arc<Pin<Box<spa::interface::r#loop::LoopControlImpl>>>,
}

pub struct Source {
    inner: Pin<Box<LoopUtilsSource>>,
}

impl Source {
    pub fn mask(&self) -> spa::flags::Io {
        self.inner.mask
    }

    fn from_loop_utils(source: Pin<Box<LoopUtilsSource>>) -> Self {
        Source { inner: source }
    }
}

pub type SourceEventFn = spa::interface::r#loop::SourceEventFn;
pub type SourceIdleFn = spa::interface::r#loop::SourceIdleFn;
pub type SourceIoFn = spa::interface::r#loop::SourceIoFn;
pub type SourceSignalFn = spa::interface::r#loop::SourceSignalFn;
pub type SourceTimerFn = spa::interface::r#loop::SourceTimerFn;

refcounted! {
    pub struct MainLoop {
        support: LoopSupport,
        // This is an atomic because it is hard to convince Rust that this will only be mutated on one
        // thread (i.e. the one on which run() is called
        running: AtomicBool,
        name: String,
        hooks: Arc<Mutex<HookList<MainLoopEvents>>>,
    }
}

impl MainLoop {
    pub fn new(props: &Properties) -> Option<MainLoop> {
        let Some(l) = InnerMainLoop::new(props) else {
            return None;
        };

        debug!("Creating main loop");

        Some(MainLoop {
            inner: new_refcounted(l),
        })
    }

    pub fn run(&self) {
        if self
            .inner
            .running
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            return;
        }

        self.inner.support.loop_control.enter();

        while self.inner.running.load(Ordering::Relaxed) {
            if let Err(res) = self
                .inner
                .support
                .loop_control
                .iterate(Some(std::time::Duration::MAX))
            {
                if res.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
            }
        }

        self.inner.support.loop_control.leave();
    }

    pub fn quit(&self) {
        debug!("quit");

        let this = self.clone();

        let stop = move |_block: bool, _seq: u32, _data: &[u8]| {
            this.inner.running.store(false, Ordering::Relaxed);
            0
        };

        let _ = self
            .inner
            .support
            .loop_
            .invoke(1, &[], false, Box::new(stop));
    }

    pub fn add_listener(&self, events: MainLoopEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    // Loop control methods
    pub fn get_fd(&self) -> RawFd {
        self.inner.support.loop_control.get_fd() as RawFd
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
    ) -> Option<Source> {
        self.inner
            .support
            .loop_utils
            .add_io(fd, mask, close, func)
            .map(Source::from_loop_utils)
    }

    pub fn update_io(&self, source: &mut Source, mask: flags::Io) -> std::io::Result<i32> {
        self.inner
            .support
            .loop_utils
            .update_io(&mut source.inner, mask)
    }

    pub fn add_idle(&self, enabled: bool, func: Box<SourceIdleFn>) -> Option<Source> {
        self.inner
            .support
            .loop_utils
            .add_idle(enabled, func)
            .map(Source::from_loop_utils)
    }

    pub fn enable_idle(&self, source: &mut Source, enabled: bool) -> std::io::Result<i32> {
        debug!("idle {enabled}");
        self.inner
            .support
            .loop_utils
            .enable_idle(&mut source.inner, enabled)
    }

    pub fn add_event(&self, func: Box<SourceEventFn>) -> Option<Source> {
        self.inner
            .support
            .loop_utils
            .add_event(func)
            .map(Source::from_loop_utils)
    }

    pub fn signal_event(&self, source: &mut Source) -> std::io::Result<i32> {
        self.inner
            .support
            .loop_utils
            .signal_event(&mut source.inner)
    }

    pub fn add_timer(&self, func: Box<SourceTimerFn>) -> Option<Source> {
        self.inner
            .support
            .loop_utils
            .add_timer(func)
            .map(Source::from_loop_utils)
    }

    pub fn update_timer(
        &self,
        source: &mut Source,
        value: &libc::timespec,
        interval: Option<&libc::timespec>,
        absolute: bool,
    ) -> std::io::Result<i32> {
        self.inner
            .support
            .loop_utils
            .update_timer(&mut source.inner, value, interval, absolute)
    }

    pub fn add_signal(&self, signal_number: i32, func: Box<SourceSignalFn>) -> Option<Source> {
        self.inner
            .support
            .loop_utils
            .add_signal(signal_number, func)
            .map(Source::from_loop_utils)
    }

    pub fn destroy_source(&self, source: Source) {
        self.inner.support.loop_utils.destroy_source(source.inner)
    }

    pub fn set_name(&mut self, name: &str) {
        debug!("main loop name {name}");
        if let Some(inner) = Arc::get_mut(&mut self.inner) {
            inner.name = name.to_string()
        }
    }

    pub(crate) fn support(&self) -> LoopSupport {
        self.inner.support.clone()
    }
}

impl Drop for InnerMainLoop {
    fn drop(&mut self) {
        self.destroy();
    }
}

impl InnerMainLoop {
    pub fn new(props: &Properties) -> Option<InnerMainLoop> {
        let support = GLOBAL_SUPPORT
            .get()
            .expect("Global support should be initialised");

        let handle = support
            .load_spa_handle(None, spa::interface::plugin::LOOP_FACTORY, None)
            .ok()?;

        let loop_ = handle.get_interface(spa::interface::LOOP).and_then(|i| {
            Arc::new(Box::into_pin(i))
                .downcast_arc_pin_box::<spa::interface::r#loop::LoopImpl>()
                .ok()
        })?;
        let loop_utils = handle
            .get_interface(spa::interface::LOOP_UTILS)
            .and_then(|i| {
                Arc::new(Box::into_pin(i))
                    .downcast_arc_pin_box::<spa::interface::r#loop::LoopUtilsImpl>()
                    .ok()
            })?;
        let loop_control = handle
            .get_interface(spa::interface::LOOP_CONTROL)
            .and_then(|i| {
                Arc::new(Box::into_pin(i))
                    .downcast_arc_pin_box::<spa::interface::r#loop::LoopControlImpl>()
                    .ok()
            })?;

        let name = if let Some(n) = props.get("loop.name") {
            n.to_string()
        } else {
            "main.loop".to_string()
        };

        Some(InnerMainLoop {
            support: LoopSupport {
                handle: Arc::new(handle),
                loop_,
                loop_utils,
                loop_control,
            },
            running: AtomicBool::new(false),
            name,
            hooks: HookList::new(),
        })
    }

    fn destroy(&self) {
        emit_hook!(self.hooks, destroy);
    }
}
