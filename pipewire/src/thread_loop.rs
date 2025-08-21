// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Sanchayan Maity

use parking_lot::lock_api::RawMutex;
use pipewire_native_spa::interface::r#loop::LoopControlHooks;
use std::cell::RefCell;
use std::sync::Arc;
use std::thread;

use crate::{
    closure, debug, default_topic, error, log, main_loop::MainLoop, new_refcounted,
    properties::Properties, refcounted, some_closure, trace,
};

default_topic!(log::topic::THREAD_LOOP);

refcounted! {
    /// Provides a main loop implementation which runs in a separate thread.
    pub struct ThreadLoop {
        thread: RefCell<Option<thread::JoinHandle<std::io::Result<i32>>>>,
        main_loop: MainLoop,
        mutex: Arc<parking_lot::Mutex<()>>,
    }
}

/// Represents an RAII-style lock on the thread main loop. Dropping this causes a thread lock taken
/// with [ThreadLoop::lock] to be unlocked.
pub struct ThreadLoopGuard {
    _guard: parking_lot::ArcMutexGuard<parking_lot::RawMutex, ()>,
}

impl ThreadLoop {
    /// Create a new thread main loop.
    pub fn new(props: &Properties) -> Option<ThreadLoop> {
        debug!("Creating thread loop");

        let inner = InnerThreadLoop::new(props)?;
        let this = ThreadLoop {
            inner: new_refcounted(inner),
        };

        let loop_support = this.inner.main_loop.support();

        loop_support.loop_control.add_hook(LoopControlHooks {
            before: some_closure!([this] {
                trace!("before");
                unsafe {
                    this.inner.mutex.raw().unlock();
                }
            }),
            after: some_closure!([this] {
                trace!("after");
                unsafe {
                    this.inner.mutex.raw().lock();
                }
            }),
        });

        Some(this)
    }

    /// Access the underlying main loop.
    ///
    /// <div class="warning">
    /// This is only intended for managing the main loop sources, and methods such as
    /// [MainLoop::run], [MainLoop::lock] etc. should not be called.
    /// </div>
    pub fn main_loop(&self) -> &MainLoop {
        &self.inner.main_loop
    }

    /// Run the thread main loop.
    pub fn run(&self) {
        debug!("run");

        let handle = thread::spawn(closure!([this <- self] {
            trace!("thread spawned");
            this.inner.main_loop.set_running()?;

            loop {
                trace!("iterate");
                let _guard = this.inner.mutex.lock();

                if let Err(e) = this.inner.main_loop.run_once() {
                    if e.kind() == std::io::ErrorKind::NotConnected {
                        error!("done: {e:?}");
                        break;
                    } else {
                        error!("terminating: {e:?}");
                        return Err(e);
                    }
                }

                // guard is dropped here, dropping the lock
            }

            trace!("thread done");

            Ok(0)
        }));

        self.inner.thread.replace(Some(handle));
    }

    /// Quit the thread main loop.
    pub fn quit(&self) {
        debug!("quit");

        if let Some(handle) = self.inner.thread.take() {
            self.inner.main_loop.quit();
            let _ = handle.join();
        }
    }

    /// Take a lock on the thread main loop. While this lock is held, the main loop thread will not
    /// run any iterations. Conversely, taking the lock will block until the current main loop
    /// iteration has completed.
    ///
    /// The lock is released when the returned [ThreadLoopGuard] is dropped.
    pub fn lock(&self) -> ThreadLoopGuard {
        debug!("lock");

        let guard = self.inner.mutex.lock_arc();
        debug!("locked");

        ThreadLoopGuard { _guard: guard }
    }
}

impl InnerThreadLoop {
    pub fn new(props: &Properties) -> Option<InnerThreadLoop> {
        let main_loop = MainLoop::new(props)?;

        Some(InnerThreadLoop {
            thread: RefCell::new(None),
            main_loop,
            mutex: Arc::new(parking_lot::Mutex::new(())),
        })
    }
}
