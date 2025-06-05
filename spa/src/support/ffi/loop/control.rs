// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Sanchayan Maity

use std::ffi::{c_int, c_uint, c_void, CString};
use std::time::Duration;

use crate::interface::ffi::{CControlHooks, CHook};
use crate::interface::r#loop::*;
use crate::interface::{self, ffi::CInterface};
use crate::support::ffi::c_string;
use crate::support::ffi::r#loop::common::{from_result, result_from};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CControlMethodsMethods {
    version: u32,

    get_fd: extern "C" fn(object: *mut c_void) -> c_uint,
    add_hook:
        extern "C" fn(object: *mut c_void, hook: &CHook, hooks: &CControlHooks, data: *mut c_void),
    enter: extern "C" fn(object: *mut c_void),
    leave: extern "C" fn(object: *mut c_void),
    iterate: extern "C" fn(object: *mut c_void, timeout: c_int) -> c_int,
    check: extern "C" fn(object: *mut c_void) -> c_int,
    lock: extern "C" fn(object: *mut c_void) -> c_int,
    unlock: extern "C" fn(object: *mut c_void) -> c_int,
    get_time:
        extern "C" fn(object: *mut c_void, abstime: *mut libc::timespec, timeout: i64) -> c_int,
    wait: extern "C" fn(object: *mut c_void, abstime: *const libc::timespec) -> c_int,
    signal: extern "C" fn(object: *mut c_void, wait_for_accept: bool) -> c_int,
    accept: extern "C" fn(object: *mut c_void) -> c_int,
}

#[repr(C)]
struct CLoopControlMethods {
    iface: CInterface,
}

struct CLoopControlMethodsImpl {}

pub fn new_impl(interface: *mut CInterface) -> LoopControlMethodsImpl {
    LoopControlMethodsImpl {
        inner: Box::pin(interface as *mut CLoopControlMethods),

        get_fd: CLoopControlMethodsImpl::get_fd,
        add_hook: CLoopControlMethodsImpl::add_hook,
        enter: CLoopControlMethodsImpl::enter,
        leave: CLoopControlMethodsImpl::leave,
        iterate: CLoopControlMethodsImpl::iterate,
        check: CLoopControlMethodsImpl::check,
        lock: CLoopControlMethodsImpl::lock,
        unlock: CLoopControlMethodsImpl::unlock,
        get_time: CLoopControlMethodsImpl::get_time,
        wait: CLoopControlMethodsImpl::wait,
        signal: CLoopControlMethodsImpl::signal,
        accept: CLoopControlMethodsImpl::accept,
    }
}

impl CLoopControlMethodsImpl {
    fn from_control_methods(this: &LoopControlMethodsImpl) -> &CLoopControlMethods {
        unsafe {
            this.inner
                .as_ref()
                .downcast_ref::<*mut CLoopControlMethods>()
                .unwrap()
                .as_ref()
                .unwrap()
        }
    }

    fn get_fd(this: &LoopControlMethodsImpl) -> u32 {
        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        unsafe { ((*funcs).get_fd)(control_impl.iface.cb.data) }
    }

    fn add_hook(this: &LoopControlMethodsImpl, hook: &CHook, hooks: &CControlHooks, data: u64) {
        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        unsafe {
            ((*funcs).add_hook)(control_impl.iface.cb.data, hook, hooks, data as *mut c_void);
        }
    }

    fn enter(this: &LoopControlMethodsImpl) {
        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        unsafe { ((*funcs).enter)(control_impl.iface.cb.data) }
    }

    fn leave(this: &LoopControlMethodsImpl) {
        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        unsafe {
            ((*funcs).leave)(control_impl.iface.cb.data);
        }
    }

    fn iterate(this: &LoopControlMethodsImpl, timeout: Option<Duration>) -> std::io::Result<i32> {
        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        let timeout: i32 = match timeout {
            Some(t) => {
                if t == Duration::MAX {
                    -1
                } else {
                    t.as_millis() as i32
                }
            }
            None => 0,
        };

        result_from(unsafe { ((*funcs).iterate)(control_impl.iface.cb.data, timeout) })
    }

    fn check(this: &LoopControlMethodsImpl) -> std::io::Result<i32> {
        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        result_from(unsafe { ((*funcs).check)(control_impl.iface.cb.data) })
    }

    fn lock(this: &LoopControlMethodsImpl) -> std::io::Result<i32> {
        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        result_from(unsafe { ((*funcs).lock)(control_impl.iface.cb.data) })
    }

    fn unlock(this: &LoopControlMethodsImpl) -> std::io::Result<i32> {
        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        result_from(unsafe { ((*funcs).lock)(control_impl.iface.cb.data) })
    }

    fn get_time(
        this: &LoopControlMethodsImpl,
        timeout: Duration,
    ) -> std::io::Result<libc::timespec> {
        let mut abstime = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        let res = unsafe {
            ((*funcs).get_time)(
                control_impl.iface.cb.data,
                &mut abstime as *mut libc::timespec,
                timeout.as_nanos() as i64,
            )
        };

        match res {
            0 => Ok(abstime),
            e => Err(std::io::Error::from_raw_os_error(-e)),
        }
    }

    fn wait(this: &LoopControlMethodsImpl, abstime: &libc::timespec) -> std::io::Result<i32> {
        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        result_from(unsafe {
            ((*funcs).wait)(control_impl.iface.cb.data, abstime as *const libc::timespec)
        })
    }

    fn signal(this: &LoopControlMethodsImpl, wait_for_accept: bool) -> std::io::Result<i32> {
        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        result_from(unsafe { ((*funcs).signal)(control_impl.iface.cb.data, wait_for_accept) })
    }

    fn accept(this: &LoopControlMethodsImpl) -> std::io::Result<i32> {
        let control_impl = Self::from_control_methods(this);
        let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

        result_from(unsafe { ((*funcs).accept)(control_impl.iface.cb.data) })
    }
}

static LOOP_CONTROL_METHODS: CControlMethodsMethods = CControlMethodsMethods {
    version: 2,

    get_fd: ControlMethodsIface::get_fd,
    add_hook: ControlMethodsIface::add_hook,
    enter: ControlMethodsIface::enter,
    leave: ControlMethodsIface::leave,
    iterate: ControlMethodsIface::iterate,
    check: ControlMethodsIface::check,
    lock: ControlMethodsIface::lock,
    unlock: ControlMethodsIface::unlock,
    get_time: ControlMethodsIface::get_time,
    wait: ControlMethodsIface::wait,
    signal: ControlMethodsIface::signal,
    accept: ControlMethodsIface::accept,
};

struct ControlMethodsIface {}

impl ControlMethodsIface {
    fn c_to_control_methods_impl(object: *mut c_void) -> &'static LoopControlMethodsImpl {
        unsafe { &*(object as *mut LoopControlMethodsImpl) }
    }

    extern "C" fn get_fd(object: *mut c_void) -> c_uint {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        control_methods_impl.get_fd()
    }

    extern "C" fn add_hook(
        object: *mut c_void,
        hook: &CHook,
        hooks: &CControlHooks,
        data: *mut c_void,
    ) {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        control_methods_impl.add_hook(hook, hooks, data as u64)
    }

    extern "C" fn enter(object: *mut c_void) {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        control_methods_impl.enter()
    }

    extern "C" fn leave(object: *mut c_void) {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        control_methods_impl.leave()
    }

    extern "C" fn iterate(object: *mut c_void, timeout: c_int) -> c_int {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        let t = if timeout == 0 {
            Duration::new(0, 0)
        } else if timeout == -1 {
            Duration::MAX
        } else {
            Duration::from_millis(timeout as u64)
        };

        from_result(control_methods_impl.iterate(Some(t)))
    }

    extern "C" fn check(object: *mut c_void) -> c_int {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        from_result(control_methods_impl.check())
    }

    extern "C" fn lock(object: *mut c_void) -> c_int {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        from_result(control_methods_impl.lock())
    }

    extern "C" fn unlock(object: *mut c_void) -> c_int {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        from_result(control_methods_impl.unlock())
    }

    extern "C" fn get_time(
        object: *mut c_void,
        abstime: *mut libc::timespec,
        timeout: i64,
    ) -> c_int {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        match control_methods_impl.get_time(Duration::from_nanos(timeout as u64)) {
            Ok(time) => {
                unsafe {
                    *abstime = time;
                };
                0
            }
            Err(e) => e.raw_os_error().unwrap(),
        }
    }

    extern "C" fn wait(object: *mut c_void, abstime: *const libc::timespec) -> c_int {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        from_result(control_methods_impl.wait(unsafe { abstime.as_ref().unwrap() }))
    }

    extern "C" fn signal(object: *mut c_void, wait_for_accept: bool) -> c_int {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        from_result(control_methods_impl.signal(wait_for_accept))
    }

    extern "C" fn accept(object: *mut c_void) -> c_int {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        from_result(control_methods_impl.accept())
    }
}

pub(crate) unsafe fn make_native(loop_ctrl: &LoopControlMethodsImpl) -> *mut CInterface {
    let c_ctrl_methods: *mut CLoopControlMethods = unsafe {
        libc::calloc(
            1,
            std::mem::size_of::<CLoopControlMethods>() as libc::size_t,
        ) as *mut CLoopControlMethods
    };
    let c_ctrl_methods = unsafe { &mut *c_ctrl_methods };

    c_ctrl_methods.iface.version = 1;
    c_ctrl_methods.iface.type_ = c_string(interface::CPU).into_raw();
    c_ctrl_methods.iface.cb.funcs =
        &LOOP_CONTROL_METHODS as *const CControlMethodsMethods as *mut c_void;
    c_ctrl_methods.iface.cb.data = loop_ctrl as *const LoopControlMethodsImpl as *mut c_void;

    c_ctrl_methods as *mut CLoopControlMethods as *mut CInterface
}

pub(crate) unsafe fn free_native(c_loop_ctrl: *mut CInterface) {
    unsafe {
        let _ = CString::from_raw((*c_loop_ctrl).type_ as *mut i8);
        libc::free(c_loop_ctrl as *mut c_void);
    }
}
