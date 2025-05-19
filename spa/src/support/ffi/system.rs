// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    ffi::{c_int, c_ulong, c_void, CString},
    os::fd::RawFd,
};

use crate::interface::{
    self,
    ffi::CInterface,
    system::{result_or_error, PollEvent, PollEvents, SystemImpl},
};

use super::c_string;

#[repr(C)]
#[derive(Copy, Clone)]
struct CSystemMethods {
    version: u32,

    /* read/write/ioctl */
    read: extern "C" fn(object: *mut c_void, fd: c_int, buf: *mut c_void, count: usize) -> isize,
    write: extern "C" fn(object: *mut c_void, fd: c_int, buf: *const c_void, count: usize) -> isize,
    ioctl: unsafe extern "C" fn(object: *mut c_void, fd: c_int, request: c_ulong, ...) -> c_int,
    close: extern "C" fn(object: *mut c_void, fd: c_int) -> c_int,

    /* clock */
    clock_gettime:
        extern "C" fn(object: *mut c_void, clockid: c_int, value: *mut libc::timespec) -> c_int,
    clock_getres:
        extern "C" fn(object: *mut c_void, clockid: c_int, res: *mut libc::timespec) -> c_int,

    /* poll */
    pollfd_create: extern "C" fn(object: *mut c_void, pfd: c_int) -> c_int,
    pollfd_add: extern "C" fn(
        object: *mut c_void,
        pfd: c_int,
        fd: c_int,
        events: PollEvents,
        data: *mut c_void,
    ) -> c_int,
    pollfd_mod: extern "C" fn(
        object: *mut c_void,
        pfd: c_int,
        fd: c_int,
        events: PollEvents,
        data: *mut c_void,
    ) -> c_int,
    pollfd_del: extern "C" fn(object: *mut c_void, pfd: c_int, fd: c_int) -> c_int,
    pollfd_wait: extern "C" fn(
        object: *mut c_void,
        pfd: c_int,
        ev: *mut PollEvent,
        n_ev: c_int,
        timeout: c_int,
    ) -> c_int,

    /* timers */
    timerfd_create: extern "C" fn(object: *mut c_void, clockid: c_int, flags: c_int) -> c_int,
    timerfd_settime: extern "C" fn(
        object: *mut c_void,
        fd: c_int,
        flags: c_int,
        new_value: *const libc::itimerspec,
        old_value: *mut libc::itimerspec,
    ) -> c_int,
    timerfd_gettime:
        extern "C" fn(object: *mut c_void, fd: c_int, curr_value: *mut libc::itimerspec) -> c_int,
    timerfd_read: extern "C" fn(object: *mut c_void, fd: c_int, expirations: *mut u64) -> c_int,

    /* events */
    eventfd_create: extern "C" fn(object: *mut c_void, flags: c_int) -> c_int,
    eventfd_write: extern "C" fn(object: *mut c_void, fd: c_int, count: u64) -> c_int,
    eventfd_read: extern "C" fn(object: *mut c_void, fd: c_int, count: *mut u64) -> c_int,

    /* signals */
    signalfd_create: extern "C" fn(object: *mut c_void, signal: c_int, flags: c_int) -> c_int,
    signalfd_read: extern "C" fn(object: *mut c_void, fd: c_int, signal: *mut c_int) -> c_int,
}

#[repr(C)]
struct CSystem {
    iface: CInterface,
}

struct CSystemImpl {}

pub fn new_impl(interface: *mut CInterface) -> SystemImpl {
    SystemImpl {
        inner: Box::pin(interface as *mut CSystem),

        read: CSystemImpl::read,
        write: CSystemImpl::write,
        /* NOTE: we can't handle varargs, so we just directly call ioctl() */
        ioctl: libc::ioctl,
        close: CSystemImpl::close,

        clock_gettime: CSystemImpl::clock_gettime,
        clock_getres: CSystemImpl::clock_getres,

        pollfd_create: CSystemImpl::pollfd_create,
        pollfd_add: CSystemImpl::pollfd_add,
        pollfd_mod: CSystemImpl::pollfd_mod,
        pollfd_del: CSystemImpl::pollfd_del,
        pollfd_wait: CSystemImpl::pollfd_wait,

        timerfd_create: CSystemImpl::timerfd_create,
        timerfd_settime: CSystemImpl::timerfd_settime,
        timerfd_gettime: CSystemImpl::timerfd_gettime,
        timerfd_read: CSystemImpl::timerfd_read,

        eventfd_create: CSystemImpl::eventfd_create,
        eventfd_read: CSystemImpl::eventfd_read,
        eventfd_write: CSystemImpl::eventfd_write,

        signalfd_create: CSystemImpl::signalfd_create,
        signalfd_read: CSystemImpl::signalfd_read,
    }
}

impl CSystemImpl {
    fn from_system(this: &SystemImpl) -> &CSystem {
        unsafe {
            this.inner
                .as_ref()
                .downcast_ref::<*const CSystem>()
                .unwrap()
                .as_ref()
                .unwrap()
        }
    }

    fn read(this: &SystemImpl, fd: RawFd, buf: &mut [u8]) -> std::io::Result<isize> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).read)(
                system.iface.cb.data,
                fd,
                buf.as_mut_ptr() as *mut c_void,
                buf.len(),
            ))
        }
    }

    fn write(this: &SystemImpl, fd: RawFd, buf: &[u8]) -> std::io::Result<isize> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).write)(
                system.iface.cb.data,
                fd,
                buf.as_ptr() as *const c_void,
                buf.len(),
            ))
        }
    }

    fn close(this: &SystemImpl, fd: RawFd) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).close)(system.iface.cb.data, fd))
        }
    }

    fn clock_gettime(
        this: &SystemImpl,
        clockid: libc::clockid_t,
        value: &mut libc::timespec,
    ) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).clock_gettime)(
                system.iface.cb.data,
                clockid,
                value,
            ))
        }
    }

    fn clock_getres(
        this: &SystemImpl,
        clockid: libc::clockid_t,
        res: &mut libc::timespec,
    ) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).clock_getres)(system.iface.cb.data, clockid, res))
        }
    }

    fn pollfd_create(this: &SystemImpl, flags: i32) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).pollfd_create)(system.iface.cb.data, flags))
        }
    }

    fn pollfd_add(
        this: &SystemImpl,
        pfd: RawFd,
        fd: RawFd,
        events: PollEvents,
        data: u64,
    ) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).pollfd_add)(
                system.iface.cb.data,
                pfd,
                fd,
                events,
                data as *mut c_void,
            ))
        }
    }

    fn pollfd_mod(
        this: &SystemImpl,
        pfd: RawFd,
        fd: RawFd,
        events: PollEvents,
        data: u64,
    ) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).pollfd_mod)(
                system.iface.cb.data,
                pfd,
                fd,
                events,
                data as *mut c_void,
            ))
        }
    }

    fn pollfd_del(this: &SystemImpl, pfd: RawFd, fd: RawFd) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).pollfd_del)(system.iface.cb.data, pfd, fd))
        }
    }

    fn pollfd_wait(
        this: &SystemImpl,
        pfd: RawFd,
        events: &mut [PollEvent],
        timeout: i32,
    ) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).pollfd_wait)(
                system.iface.cb.data,
                pfd,
                events.as_mut_ptr(),
                events.len() as i32,
                timeout,
            ))
        }
    }

    fn timerfd_create(
        this: &SystemImpl,
        clockid: libc::clockid_t,
        flags: i32,
    ) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).timerfd_create)(
                system.iface.cb.data,
                clockid,
                flags,
            ))
        }
    }

    fn timerfd_settime(
        this: &SystemImpl,
        fd: RawFd,
        flags: i32,
        new_value: &libc::itimerspec,
        old_value: &mut libc::itimerspec,
    ) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).timerfd_settime)(
                system.iface.cb.data,
                fd,
                flags,
                new_value,
                old_value,
            ))
        }
    }

    fn timerfd_gettime(
        this: &SystemImpl,
        fd: RawFd,
        curr_value: &mut libc::itimerspec,
    ) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).timerfd_gettime)(
                system.iface.cb.data,
                fd,
                curr_value,
            ))
        }
    }

    fn timerfd_read(this: &SystemImpl, fd: RawFd) -> std::io::Result<u64> {
        let mut expirations: u64 = 0;
        let res = unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            ((*funcs).timerfd_read)(system.iface.cb.data, fd, &mut expirations)
        };

        if res < 0 {
            Err(std::io::Error::last_os_error())
        } else if res != 8 {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "read timerfd returned unexpected size",
            ))
        } else {
            Ok(expirations)
        }
    }

    fn eventfd_create(this: &SystemImpl, flags: i32) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).eventfd_create)(system.iface.cb.data, flags))
        }
    }

    fn eventfd_read(this: &SystemImpl, fd: RawFd) -> std::io::Result<u64> {
        let mut buf = 0u64;
        let res = unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            ((*funcs).eventfd_read)(system.iface.cb.data, fd, &mut buf)
        };

        if res < 0 {
            Err(std::io::Error::last_os_error())
        } else if res != 8 {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "read eventfd returned unexpected size",
            ))
        } else {
            Ok(buf)
        }
    }

    fn eventfd_write(this: &SystemImpl, fd: RawFd, value: u64) -> std::io::Result<i32> {
        let res = unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            ((*funcs).eventfd_write)(system.iface.cb.data, fd, value)
        };

        if res < 0 {
            Err(std::io::Error::last_os_error())
        } else if res != 8 {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "write eventfd returned unexpected size",
            ))
        } else {
            Ok(0)
        }
    }

    fn signalfd_create(this: &SystemImpl, signal: u32, flags: i32) -> std::io::Result<i32> {
        unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            result_or_error(((*funcs).signalfd_create)(
                system.iface.cb.data,
                signal as c_int,
                flags,
            ))
        }
    }

    fn signalfd_read(this: &SystemImpl, fd: RawFd) -> std::io::Result<u32> {
        let mut signal = 0u32;

        let res = unsafe {
            let system = Self::from_system(this);
            let funcs = system.iface.cb.funcs as *const CSystemMethods;

            ((*funcs).signalfd_read)(
                system.iface.cb.data,
                fd,
                &mut signal as *mut u32 as *mut c_int,
            )
        };

        if res < 0 {
            Err(std::io::Error::last_os_error())
        } else if res != 8 {
            Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "read signalfd returned unexpected size",
            ))
        } else {
            Ok(signal)
        }
    }
}

pub fn make_native(system: &SystemImpl) -> *mut CInterface {
    let c_system: *mut CSystem =
        unsafe { libc::calloc(1, std::mem::size_of::<CSystem>() as libc::size_t) as *mut CSystem };
    let c_system = unsafe { &mut *c_system };

    c_system.iface.version = 0;
    c_system.iface.type_ = c_string(interface::LOG).into_raw();
    c_system.iface.cb.funcs = &SYSTEM_METHODS as *const CSystemMethods as *mut c_void;
    c_system.iface.cb.data = system as *const SystemImpl as *mut c_void;

    c_system as *mut CSystem as *mut CInterface
}

pub fn free_native(c_system: *mut CInterface) {
    unsafe {
        let _ = CString::from_raw((*c_system).type_ as *mut i8);
        libc::free(c_system as *mut c_void);
    }
}

static SYSTEM_METHODS: CSystemMethods = CSystemMethods {
    version: 0,

    read: SystemImplIface::read,
    write: SystemImplIface::write,
    /* NOTE: we can't handle varargs, so we just directly call ioctl() */
    ioctl: impl_ioctl,
    close: SystemImplIface::close,

    clock_gettime: SystemImplIface::clock_gettime,
    clock_getres: SystemImplIface::clock_getres,

    pollfd_create: SystemImplIface::pollfd_create,
    pollfd_add: SystemImplIface::pollfd_add,
    pollfd_mod: SystemImplIface::pollfd_mod,
    pollfd_del: SystemImplIface::pollfd_del,
    pollfd_wait: SystemImplIface::pollfd_wait,

    timerfd_create: SystemImplIface::timerfd_create,
    timerfd_settime: SystemImplIface::timerfd_settime,
    timerfd_gettime: SystemImplIface::timerfd_gettime,
    timerfd_read: SystemImplIface::timerfd_read,

    eventfd_create: SystemImplIface::eventfd_create,
    eventfd_read: SystemImplIface::eventfd_read,
    eventfd_write: SystemImplIface::eventfd_write,

    signalfd_create: SystemImplIface::signalfd_create,
    signalfd_read: SystemImplIface::signalfd_read,
};

struct SystemImplIface {}

extern "C" {
    fn impl_ioctl(object: *mut c_void, fd: c_int, request: c_ulong, ...) -> c_int;
}

impl SystemImplIface {
    fn c_to_system_impl(object: *mut c_void) -> &'static SystemImpl {
        unsafe { &*(object as *mut SystemImpl) }
    }

    extern "C" fn read(object: *mut c_void, fd: c_int, buf: *mut c_void, count: usize) -> isize {
        let system = Self::c_to_system_impl(object);
        let buf = unsafe { std::slice::from_raw_parts_mut(buf as *mut u8, count) };

        let res = system.read(fd, buf);

        res.unwrap_or(-1)
    }

    extern "C" fn write(object: *mut c_void, fd: c_int, buf: *const c_void, count: usize) -> isize {
        let system = Self::c_to_system_impl(object);
        let buf = unsafe { std::slice::from_raw_parts(buf as *const u8, count) };

        let res = system.write(fd, buf);

        res.unwrap_or(-1)
    }

    extern "C" fn close(object: *mut c_void, fd: c_int) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.close(fd);

        res.unwrap_or(-1)
    }

    extern "C" fn clock_gettime(
        object: *mut c_void,
        clockid: c_int,
        value: *mut libc::timespec,
    ) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = unsafe { system.clock_gettime(clockid, value.as_mut().unwrap()) };

        res.unwrap_or(-1)
    }

    extern "C" fn clock_getres(
        object: *mut c_void,
        clockid: c_int,
        res: *mut libc::timespec,
    ) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = unsafe { system.clock_getres(clockid, res.as_mut().unwrap()) };

        res.unwrap_or(-1)
    }

    extern "C" fn pollfd_create(object: *mut c_void, flags: c_int) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.pollfd_create(flags);

        res.unwrap_or(-1)
    }

    extern "C" fn pollfd_add(
        object: *mut c_void,
        pfd: c_int,
        fd: c_int,
        events: PollEvents,
        data: *mut c_void,
    ) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.pollfd_add(pfd, fd, events, data as u64);

        res.unwrap_or(-1)
    }

    extern "C" fn pollfd_mod(
        object: *mut c_void,
        pfd: c_int,
        fd: c_int,
        events: PollEvents,
        data: *mut c_void,
    ) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.pollfd_mod(pfd, fd, events, data as u64);

        res.unwrap_or(-1)
    }

    extern "C" fn pollfd_del(object: *mut c_void, pfd: c_int, fd: c_int) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.pollfd_del(pfd, fd);

        res.unwrap_or(-1)
    }

    extern "C" fn pollfd_wait(
        object: *mut c_void,
        pfd: c_int,
        ev: *mut PollEvent,
        n_ev: c_int,
        timeout: c_int,
    ) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.pollfd_wait(
            pfd,
            unsafe { std::slice::from_raw_parts_mut(ev, n_ev as usize) },
            timeout,
        );

        res.unwrap_or(-1)
    }

    extern "C" fn timerfd_create(object: *mut c_void, clockid: c_int, flags: c_int) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.timerfd_create(clockid, flags);

        res.unwrap_or(-1)
    }

    extern "C" fn timerfd_settime(
        object: *mut c_void,
        fd: c_int,
        flags: c_int,
        new_value: *const libc::itimerspec,
        old_value: *mut libc::itimerspec,
    ) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.timerfd_settime(fd, flags, unsafe { &*new_value }, unsafe {
            &mut *old_value
        });

        res.unwrap_or(-1)
    }

    extern "C" fn timerfd_gettime(
        object: *mut c_void,
        fd: c_int,
        curr_value: *mut libc::itimerspec,
    ) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.timerfd_gettime(fd, unsafe { &mut *curr_value });

        res.unwrap_or(-1)
    }

    extern "C" fn timerfd_read(object: *mut c_void, fd: c_int, expirations: *mut u64) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.timerfd_read(fd);

        match res {
            Ok(exp) => {
                unsafe { *expirations = exp };
                0
            }
            Err(_) => -1,
        }
    }

    extern "C" fn eventfd_create(object: *mut c_void, flags: c_int) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.eventfd_create(flags);

        res.unwrap_or(-1)
    }

    extern "C" fn eventfd_read(object: *mut c_void, fd: c_int, count: *mut u64) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.eventfd_read(fd);

        match res {
            Ok(count_val) => {
                unsafe { *count = count_val };
                0
            }
            Err(_) => -1,
        }
    }

    extern "C" fn eventfd_write(object: *mut c_void, fd: c_int, count: u64) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.eventfd_write(fd, count);

        res.unwrap_or(-1)
    }

    extern "C" fn signalfd_create(object: *mut c_void, signal: c_int, flags: c_int) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.signalfd_create(signal as u32, flags);

        res.unwrap_or(-1)
    }

    extern "C" fn signalfd_read(object: *mut c_void, fd: c_int, signal: *mut c_int) -> i32 {
        let system = Self::c_to_system_impl(object);

        let res = system.signalfd_read(fd);

        match res {
            Ok(sig) => {
                unsafe { *signal = sig as c_int };
                0
            }
            Err(_) => -1,
        }
    }
}
