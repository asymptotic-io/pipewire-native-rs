// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Sanchayan Maity

use std::ffi::{c_int, c_uint, c_void, CString};

use crate::interface::ffi::{CControlHooks, CHook};
use crate::interface::r#loop::*;
use crate::interface::{self, ffi::CInterface};

use crate::support::ffi::c_string;
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CControlMethodsMethods {
    version: u32,

    get_fd: extern "C" fn(object: *mut c_void) -> c_uint,
    add_hook:
        extern "C" fn(object: *mut c_void, hook: &CHook, hooks: &CControlHooks, data: *mut c_void),
    enter: extern "C" fn(object: *mut c_void),
    leave: extern "C" fn(object: *mut c_void),
    iterate: extern "C" fn(object: *mut c_void) -> c_int,
    check: extern "C" fn(object: *mut c_void) -> c_int,
}

#[repr(C)]
struct CControlMethods {
    iface: CInterface,
}

struct CControlMethodsImpl {}

pub fn new_impl(interface: *mut CInterface) -> LoopControlMethodsImpl {
    LoopControlMethodsImpl {
        inner: Box::pin(interface as *mut CControlMethods),

        get_fd: CControlMethodsImpl::get_fd,
        add_hook: CControlMethodsImpl::add_hook,
        enter: CControlMethodsImpl::enter,
        leave: CControlMethodsImpl::leave,
        iterate: CControlMethodsImpl::iterate,
        check: CControlMethodsImpl::check,
    }
}

impl CControlMethodsImpl {
    fn from_control_methods(this: &LoopControlMethodsImpl) -> &CControlMethods {
        unsafe {
            this.inner
                .as_ref()
                .downcast_ref::<*const CControlMethods>()
                .unwrap()
                .as_ref()
                .unwrap()
        }
    }

    fn get_fd(this: &LoopControlMethodsImpl) -> u32 {
        unsafe {
            let control_impl = Self::from_control_methods(this);
            let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

            ((*funcs).get_fd)(control_impl.iface.cb.data)
        }
    }

    fn add_hook(this: &LoopControlMethodsImpl, hook: &CHook, hooks: &CControlHooks, data: u64) {
        unsafe {
            let control_impl = Self::from_control_methods(this);
            let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

            ((*funcs).add_hook)(control_impl.iface.cb.data, hook, hooks, data as *mut c_void);
        }
    }

    fn enter(this: &LoopControlMethodsImpl) {
        unsafe {
            let control_impl = Self::from_control_methods(this);
            let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

            ((*funcs).enter)(control_impl.iface.cb.data)
        }
    }

    fn leave(this: &LoopControlMethodsImpl) {
        unsafe {
            let control_impl = Self::from_control_methods(this);
            let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

            ((*funcs).leave)(control_impl.iface.cb.data);
        }
    }

    fn iterate(this: &LoopControlMethodsImpl) -> i32 {
        unsafe {
            let control_impl = Self::from_control_methods(this);
            let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

            ((*funcs).iterate)(control_impl.iface.cb.data)
        }
    }

    fn check(this: &LoopControlMethodsImpl) -> i32 {
        unsafe {
            let control_impl = Self::from_control_methods(this);
            let funcs = control_impl.iface.cb.funcs as *const CControlMethodsMethods;

            ((*funcs).check)(control_impl.iface.cb.data)
        }
    }
}

static LOOP_CONTROL_METHODS: CControlMethodsMethods = CControlMethodsMethods {
    version: 0,

    get_fd: ControlMethodsIface::get_fd,
    add_hook: ControlMethodsIface::add_hook,
    enter: ControlMethodsIface::enter,
    leave: ControlMethodsIface::leave,
    iterate: ControlMethodsIface::iterate,
    check: ControlMethodsIface::check,
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

    extern "C" fn iterate(object: *mut c_void) -> c_int {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        control_methods_impl.iterate()
    }

    extern "C" fn check(object: *mut c_void) -> c_int {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        control_methods_impl.check()
    }
}

pub(crate) unsafe fn make_native(loop_ctrl: &LoopControlMethodsImpl) -> *mut CInterface {
    let c_ctrl_methods: *mut CControlMethods = unsafe {
        libc::calloc(1, std::mem::size_of::<CControlMethods>() as libc::size_t)
            as *mut CControlMethods
    };
    let c_ctrl_methods = unsafe { &mut *c_ctrl_methods };

    c_ctrl_methods.iface.version = 1;
    c_ctrl_methods.iface.type_ = c_string(interface::CPU).into_raw();
    c_ctrl_methods.iface.cb.funcs =
        &LOOP_CONTROL_METHODS as *const CControlMethodsMethods as *mut c_void;
    c_ctrl_methods.iface.cb.data = loop_ctrl as *const LoopControlMethodsImpl as *mut c_void;

    c_ctrl_methods as *mut CControlMethods as *mut CInterface
}

pub(crate) unsafe fn free_native(c_loop_ctrl: *mut CInterface) {
    unsafe {
        let _ = CString::from_raw((*c_loop_ctrl).type_ as *mut i8);
        libc::free(c_loop_ctrl as *mut c_void);
    }
}
