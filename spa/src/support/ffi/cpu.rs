// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Sanchayan Maity

use std::ffi::{c_int, c_uint, c_void, CString};

use crate::interface;
use crate::interface::cpu::{CpuFlags, CpuImpl, CpuVm};
use crate::interface::ffi::CInterface;

use super::c_string;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CCpuMethods {
    version: u32,

    get_flags: extern "C" fn(object: *mut c_void) -> c_uint,
    force_flags: extern "C" fn(object: *mut c_void, flags: c_uint) -> c_int,
    get_count: extern "C" fn(object: *mut c_void) -> c_uint,
    get_max_align: extern "C" fn(object: *mut c_void) -> c_uint,
    get_vm_type: extern "C" fn(object: *mut c_void) -> c_uint,
    zero_denormals: extern "C" fn(object: *mut c_void, flags: bool) -> c_int,
}

#[repr(C)]
struct CCpu {
    iface: CInterface,
}

struct CCpuImpl {}

pub fn new_impl(interface: *mut CInterface) -> CpuImpl {
    CpuImpl {
        inner: Box::pin(interface as *mut CCpu),

        get_flags: CCpuImpl::get_flags,
        force_flags: CCpuImpl::force_flags,
        get_count: CCpuImpl::get_count,
        get_max_align: CCpuImpl::get_max_align,
        get_vm_type: CCpuImpl::get_vm_type,
        zero_denormals: CCpuImpl::zero_denormals,
    }
}

impl CCpuImpl {
    fn from_cpu(this: &CpuImpl) -> &CCpu {
        unsafe {
            this.inner
                .as_ref()
                .downcast_ref::<*const CCpu>()
                .unwrap()
                .as_ref()
                .unwrap()
        }
    }

    fn get_flags(this: &CpuImpl) -> CpuFlags {
        unsafe {
            let cpu = Self::from_cpu(this);
            let funcs = cpu.iface.cb.funcs as *const CCpuMethods;

            CpuFlags::try_from(((*funcs).get_flags)(cpu.iface.cb.data)).unwrap()
        }
    }

    fn force_flags(this: &CpuImpl, flags: CpuFlags) -> i32 {
        unsafe {
            let cpu = Self::from_cpu(this);
            let funcs = cpu.iface.cb.funcs as *const CCpuMethods;

            ((*funcs).force_flags)(cpu.iface.cb.data, flags as u32)
        }
    }

    fn get_count(this: &CpuImpl) -> u32 {
        unsafe {
            let cpu = Self::from_cpu(this);
            let funcs = cpu.iface.cb.funcs as *const CCpuMethods;

            ((*funcs).get_count)(cpu.iface.cb.data)
        }
    }

    fn get_max_align(this: &CpuImpl) -> u32 {
        unsafe {
            let cpu = Self::from_cpu(this);
            let funcs = cpu.iface.cb.funcs as *const CCpuMethods;

            ((*funcs).get_max_align)(cpu.iface.cb.data)
        }
    }

    fn get_vm_type(this: &CpuImpl) -> CpuVm {
        unsafe {
            let cpu = Self::from_cpu(this);
            let funcs = cpu.iface.cb.funcs as *const CCpuMethods;

            CpuVm::try_from(((*funcs).get_vm_type)(cpu.iface.cb.data))
                .expect("Expected valid VM type")
        }
    }

    fn zero_denormals(this: &CpuImpl, enable: bool) -> i32 {
        unsafe {
            let cpu = Self::from_cpu(this);
            let funcs = cpu.iface.cb.funcs as *const CCpuMethods;

            ((*funcs).zero_denormals)(cpu.iface.cb.data, enable)
        }
    }
}

struct CpuImplIface {}

impl CpuImplIface {
    fn c_to_cpu_impl(object: *mut c_void) -> &'static mut CpuImpl {
        unsafe { (object as *mut CpuImpl).as_mut().unwrap() }
    }

    extern "C" fn get_flags(object: *mut c_void) -> c_uint {
        let cpu_impl = Self::c_to_cpu_impl(object);

        cpu_impl.get_flags() as u32
    }

    extern "C" fn force_flags(object: *mut c_void, flags: c_uint) -> c_int {
        let cpu_impl = Self::c_to_cpu_impl(object);
        let f = CpuFlags::try_from(flags).expect("Expected valid CPU flags");

        cpu_impl.force_flags(f)
    }

    extern "C" fn get_count(object: *mut c_void) -> c_uint {
        let cpu_impl = Self::c_to_cpu_impl(object);

        cpu_impl.get_count()
    }

    extern "C" fn get_max_align(object: *mut c_void) -> c_uint {
        let cpu_impl = Self::c_to_cpu_impl(object);

        cpu_impl.get_max_align()
    }

    extern "C" fn get_vm_type(object: *mut c_void) -> c_uint {
        let cpu_impl = Self::c_to_cpu_impl(object);

        cpu_impl.get_vm_type() as u32
    }

    extern "C" fn zero_denormals(object: *mut c_void, flags: bool) -> c_int {
        let cpu_impl = Self::c_to_cpu_impl(object);

        cpu_impl.zero_denormals(flags)
    }
}

static CPU_METHODS: CCpuMethods = CCpuMethods {
    version: 0,

    get_flags: CpuImplIface::get_flags,
    force_flags: CpuImplIface::force_flags,
    get_count: CpuImplIface::get_count,
    get_max_align: CpuImplIface::get_max_align,
    get_vm_type: CpuImplIface::get_vm_type,
    zero_denormals: CpuImplIface::zero_denormals,
};

pub fn make_native(cpu: &CpuImpl) -> *mut CInterface {
    let c_cpu: *mut CCpu =
        unsafe { libc::calloc(1, std::mem::size_of::<CCpu>() as libc::size_t) as *mut CCpu };
    let c_cpu = unsafe { &mut *c_cpu };

    c_cpu.iface.version = 0;
    c_cpu.iface.type_ = c_string(interface::CPU).into_raw();
    c_cpu.iface.cb.funcs = &CPU_METHODS as *const CCpuMethods as *mut c_void;
    c_cpu.iface.cb.data = cpu as *const CpuImpl as *mut c_void;

    c_cpu as *mut CCpu as *mut CInterface
}

pub fn free_native(c_cpu: *mut CInterface) {
    unsafe {
        let _ = CString::from_raw((*c_cpu).type_ as *mut i8);
        libc::free(c_cpu as *mut c_void);
    }
}
