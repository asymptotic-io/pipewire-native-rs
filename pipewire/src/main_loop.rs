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

/// Main loop events
pub struct MainLoopEvents {
    /// The main loop was destroyed.
    pub destroy: Option<Box<dyn FnMut()>>,
}

unsafe impl Send for MainLoopEvents {}

#[derive(Clone)]
pub(crate) struct LoopSupport {
    #[allow(dead_code)]
    handle: Arc<Box<dyn spa::interface::plugin::Handle + Send + Sync>>,
    pub(crate) loop_: Arc<Pin<Box<spa::interface::r#loop::LoopImpl>>>,
    pub(crate) loop_utils: Arc<Pin<Box<spa::interface::r#loop::LoopUtilsImpl>>>,
    pub(crate) loop_control: Arc<Pin<Box<spa::interface::r#loop::LoopControlImpl>>>,
}

/// An event source on the mainloop.
pub struct Source {
    inner: Pin<Box<LoopUtilsSource>>,
}

impl Source {
    /// Mask representing current set of even types the source is interested in.
    pub fn mask(&self) -> spa::flags::Io {
        self.inner.mask
    }

    fn from_loop_utils(source: Pin<Box<LoopUtilsSource>>) -> Self {
        Source { inner: source }
    }
}

/// Callback for events triggered by [MainLoop::signal_event]
pub type SourceEventFn = spa::interface::r#loop::SourceEventFn;
/// Callback for events triggered by [MainLoop] idle events.
pub type SourceIdleFn = spa::interface::r#loop::SourceIdleFn;
/// Callback for events triggered by [MainLoop] I/O events.
pub type SourceIoFn = spa::interface::r#loop::SourceIoFn;
/// Callback for events triggered by signals on the [MainLoop].
pub type SourceSignalFn = spa::interface::r#loop::SourceSignalFn;
/// Callback for timer events on the [MainLoop].
pub type SourceTimerFn = spa::interface::r#loop::SourceTimerFn;

refcounted! {
    /// An event loop implementation. This event loop is used for communication with PipeWire, and
    /// provides a number of primitives for integration with applications and any other event loops
    /// or event sources they might have.
    ///
    /// Many of these primitives are quite low-level, and it is also possible to use the main loop
    /// only for PipeWire-related events. This can be done by creating the [MainLoop] and invoking
    /// [MainLoop::run] on a separate thread. In this case, it is important to make sure that any
    /// access to common data structures is protected from concurrent access. The [MainLoop::lock]
    /// and [MainLoop::unlock] methods provide such a mechanism.
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
    /// Create a new main loop.
    pub fn new(props: &Properties) -> Option<MainLoop> {
        let l = InnerMainLoop::new(props)?;

        debug!("Creating main loop");

        Some(MainLoop {
            inner: new_refcounted(l),
        })
    }

    pub(crate) fn set_running(&self) -> std::io::Result<()> {
        if self
            .inner
            .running
            .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
            .is_err()
        {
            Err(std::io::Error::from(std::io::ErrorKind::AlreadyExists))
        } else {
            Ok(())
        }
    }

    pub(crate) fn run_once(&self) -> std::io::Result<i32> {
        if !self.inner.running.load(Ordering::Relaxed) {
            return Err(std::io::Error::from(std::io::ErrorKind::NotConnected));
        }

        self.inner.support.loop_control.enter();

        let res = self
            .inner
            .support
            .loop_control
            .iterate(Some(std::time::Duration::MAX));

        self.inner.support.loop_control.leave();

        res
    }

    /// Run the main loop. This takes over executation of the current thread until [MainLoop::quit]
    /// is invoked.
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

    /// Terminate execution of the main loop.
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

    /// Add a listener for main loop events.
    pub fn add_listener(&self, events: MainLoopEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    // Loop control methods
    // TODO: decide how we want to expose these, if at all
    #[doc(hidden)]
    pub fn get_fd(&self) -> RawFd {
        self.inner.support.loop_control.get_fd() as RawFd
    }

    #[doc(hidden)]
    pub fn enter(&self) {
        trace!("enter");
        self.inner.support.loop_control.enter()
    }

    #[doc(hidden)]
    pub fn leave(&self) {
        trace!("leave");
        self.inner.support.loop_control.leave()
    }

    /// Run one iteration of the loop, returning if no events occurred in `timeout` time. A value
    /// of `None` signifies an infinite timeout. Returns the number of iterations performed.
    pub fn iterate(&self, timeout: Option<std::time::Duration>) -> std::io::Result<i32> {
        trace!("iterate");
        self.inner.support.loop_control.iterate(timeout)
    }

    #[doc(hidden)]
    pub fn check(&self) -> std::io::Result<i32> {
        self.inner.support.loop_control.check()
    }

    /// Take the main loop lock, preventing it from running until the lock is released. The main
    /// loop takes this lock in each of its iterations.
    pub fn lock(&self) -> std::io::Result<()> {
        trace!("lock");
        self.inner.support.loop_control.lock()?;
        Ok(())
    }

    /// Release the main loop lock, allowing it to run again.
    pub fn unlock(&self) -> std::io::Result<()> {
        trace!("unlock");
        self.inner.support.loop_control.unlock()?;
        Ok(())
    }

    /// Get the time corresponding to the specified timeout, which can be used with
    /// [MainLoop::wait].
    pub fn get_time(&self, timeout: std::time::Duration) -> std::io::Result<libc::timespec> {
        self.inner.support.loop_control.get_time(timeout)
    }

    /// Wait until the specified time or until signalled.
    pub fn wait(&self, abstime: &libc::timespec) -> std::io::Result<()> {
        debug!("wait");
        self.inner.support.loop_control.wait(abstime)?;
        Ok(())
    }

    /// Signal any threads waiting with [MainLoop::wait] to wake up.
    pub fn signal(&self, wait_for_accept: bool) -> std::io::Result<()> {
        debug!("signal");
        self.inner.support.loop_control.signal(wait_for_accept)?;
        Ok(())
    }

    /// Acknowledge a [MainLoop::signal] with `wait_for_accept: true`, allowing that thread to
    /// proceed.
    pub fn accept(&self) -> std::io::Result<()> {
        debug!("accept");
        self.inner.support.loop_control.accept()?;
        Ok(())
    }

    /// Add an I/O event source using the given `fd`.
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

    /// Update the set of events of interest (given by `mask`).
    pub fn update_io(&self, source: &mut Source, mask: flags::Io) -> std::io::Result<i32> {
        self.inner
            .support
            .loop_utils
            .update_io(&mut source.inner, mask)
    }

    /// Add an idle event source (called after each iteration of the main loop).
    pub fn add_idle(&self, enabled: bool, func: Box<SourceIdleFn>) -> Option<Source> {
        self.inner
            .support
            .loop_utils
            .add_idle(enabled, func)
            .map(Source::from_loop_utils)
    }

    /// Enable or disable an idle event source.
    pub fn enable_idle(&self, source: &mut Source, enabled: bool) -> std::io::Result<i32> {
        debug!("idle {enabled}");
        self.inner
            .support
            .loop_utils
            .enable_idle(&mut source.inner, enabled)
    }

    /// Add a source which can be triggered using [Self::signal_event].
    pub fn add_event(&self, func: Box<SourceEventFn>) -> Option<Source> {
        self.inner
            .support
            .loop_utils
            .add_event(func)
            .map(Source::from_loop_utils)
    }

    /// Signal a source added with [Self::add_event].
    pub fn signal_event(&self, source: &mut Source) -> std::io::Result<i32> {
        self.inner
            .support
            .loop_utils
            .signal_event(&mut source.inner)
    }

    /// Add a timer source.
    pub fn add_timer(&self, func: Box<SourceTimerFn>) -> Option<Source> {
        self.inner
            .support
            .loop_utils
            .add_timer(func)
            .map(Source::from_loop_utils)
    }

    /// Update the timer interval of a timer source.
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

    /// Add a source triggered by UNIX signals.
    pub fn add_signal(&self, signal_number: i32, func: Box<SourceSignalFn>) -> Option<Source> {
        self.inner
            .support
            .loop_utils
            .add_signal(signal_number, func)
            .map(Source::from_loop_utils)
    }

    /// Remove and destroy an event source.
    pub fn destroy_source(&self, source: Source) {
        self.inner.support.loop_utils.destroy_source(source.inner)
    }

    /// Set the main loop name.
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
