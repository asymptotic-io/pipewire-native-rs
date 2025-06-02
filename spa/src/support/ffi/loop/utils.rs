// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan
// SPDX-FileCopyrightText: Copyright (c) 2025 Sanchayan Maity
//
use std::{
    collections::HashMap,
    ffi::{c_int, c_uint, c_ulong, c_void, CString},
    os::fd::RawFd,
    pin::Pin,
};

use crate::interface::ffi::CSource;
use crate::interface::r#loop::*;
use crate::interface::{self, ffi::CInterface};

use crate::support::ffi::c_string;

use super::result_from;

type CSourceIoFn = extern "C" fn(data: *mut c_void, fd: c_int, mask: c_uint);
type CSourceIdleFn = extern "C" fn(data: *mut c_void);
type CSourceEventFn = extern "C" fn(data: *mut c_void, count: c_ulong);
type CSourceTimerFn = extern "C" fn(data: *mut c_void, expirations: c_ulong);
type CSourceSignalFn = extern "C" fn(data: *mut c_void, signal_number: c_int);

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CLoopUtilsMethods {
    version: u32,

    add_io: extern "C" fn(
        object: *mut c_void,
        fd: c_int,
        mask: u32,
        close: bool,
        func: CSourceIoFn,
        data: *mut c_void,
    ) -> *mut CSource,
    update_io: extern "C" fn(object: *mut c_void, source: *mut CSource, mask: u32) -> c_int,
    add_idle: extern "C" fn(
        object: *mut c_void,
        enabled: bool,
        func: CSourceIdleFn,
        data: *mut c_void,
    ) -> *mut CSource,
    enable_idle: extern "C" fn(object: *mut c_void, source: *mut CSource, enabled: bool) -> c_int,
    add_event:
        extern "C" fn(object: *mut c_void, func: CSourceEventFn, data: *mut c_void) -> *mut CSource,
    signal_event: extern "C" fn(object: *mut c_void, source: *mut CSource) -> c_int,
    add_timer:
        extern "C" fn(object: *mut c_void, func: CSourceTimerFn, data: *mut c_void) -> *mut CSource,
    update_timer: extern "C" fn(
        object: *mut c_void,
        source: *mut CSource,
        value: *const libc::timespec,
        interval: *const libc::timespec,
        absolute: bool,
    ) -> c_int,
    add_signal: extern "C" fn(
        object: *mut c_void,
        signal_number: c_int,
        func: CSourceSignalFn,
        data: *mut c_void,
    ) -> *mut CSource,
    destroy_source: extern "C" fn(object: *mut c_void, source: *mut CSource),
}

struct CLoopUtils {
    iface: CInterface,
}

struct CLoopUtilsImpl {}

pub fn new_impl(interface: *mut CInterface) -> LoopUtilsImpl {
    LoopUtilsImpl {
        inner: Box::pin(interface as *mut CLoopUtils),

        add_io: CLoopUtilsImpl::add_io,
        update_io: CLoopUtilsImpl::update_io,
        add_idle: CLoopUtilsImpl::add_idle,
        enable_idle: CLoopUtilsImpl::enable_idle,
        add_event: CLoopUtilsImpl::add_event,
        signal_event: CLoopUtilsImpl::signal_event,
        add_timer: CLoopUtilsImpl::add_timer,
        update_timer: CLoopUtilsImpl::update_timer,
        add_signal: CLoopUtilsImpl::add_signal,
        destroy_source: CLoopUtilsImpl::destroy_source,
    }
}

impl CLoopUtilsImpl {
    fn from_loop_utils(this: &LoopUtilsImpl) -> &CLoopUtils {
        unsafe {
            this.inner
                .as_ref()
                .downcast_ref::<*mut CLoopUtils>()
                .unwrap()
                .as_ref()
                .unwrap()
        }
    }

    #[no_mangle]
    extern "C" fn source_io_trampoline(data: *mut c_void, fd: c_int, mask: c_uint) {
        let source = unsafe { (data as *mut LoopUtilsSource).as_mut().unwrap() };

        let func = match source.cb {
            LoopUtilsSourceCb::Io(ref mut f) => f,
            _ => panic!("source_io_trampoline called with non-IO source"),
        };

        (func)(fd, mask)
    }

    fn add_io(
        this: &LoopUtilsImpl,
        fd: RawFd,
        mask: u32,
        close: bool,
        func: Box<SourceIoFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        let loop_utils = Self::from_loop_utils(this);
        let funcs = loop_utils.iface.cb.funcs as *const CLoopUtilsMethods;

        let mut source = Box::pin(LoopUtilsSource {
            cb: LoopUtilsSourceCb::Io(func),
            inner: Box::new(std::ptr::null_mut::<CSource>()),
        });

        let c_source = unsafe {
            ((*funcs).add_io)(
                loop_utils.iface.cb.data,
                fd,
                mask,
                close,
                Self::source_io_trampoline,
                &mut *source as *mut LoopUtilsSource as *mut c_void,
            )
        };

        if c_source.is_null() {
            None
        } else {
            source.inner = Box::new(c_source);
            Some(source)
        }
    }

    fn update_io(
        this: &LoopUtilsImpl,
        source: &mut Pin<Box<LoopUtilsSource>>,
        mask: u32,
    ) -> std::io::Result<i32> {
        let loop_utils = Self::from_loop_utils(this);
        let funcs = loop_utils.iface.cb.funcs as *const CLoopUtilsMethods;
        let source = *source.inner.downcast_ref::<*mut CSource>().unwrap();

        result_from(unsafe { ((*funcs).update_io)(loop_utils.iface.cb.data, source, mask) })
    }

    #[no_mangle]
    extern "C" fn source_idle_trampoline(data: *mut c_void) {
        let source = unsafe { (data as *mut LoopUtilsSource).as_mut().unwrap() };

        let func = match source.cb {
            LoopUtilsSourceCb::Idle(ref mut f) => f,
            _ => panic!("source_idle_trampoline called with non-idle source"),
        };

        (func)()
    }

    fn add_idle(
        this: &LoopUtilsImpl,
        enabled: bool,
        func: Box<SourceIdleFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        let loop_utils = Self::from_loop_utils(this);
        let funcs = loop_utils.iface.cb.funcs as *const CLoopUtilsMethods;

        let mut source = Box::pin(LoopUtilsSource {
            cb: LoopUtilsSourceCb::Idle(func),
            inner: Box::new(std::ptr::null_mut::<CSource>()),
        });

        let c_source = unsafe {
            ((*funcs).add_idle)(
                loop_utils.iface.cb.data,
                enabled,
                Self::source_idle_trampoline,
                &mut *source as *mut LoopUtilsSource as *mut c_void,
            )
        };

        if c_source.is_null() {
            None
        } else {
            source.inner = Box::new(c_source);
            Some(source)
        }
    }

    fn enable_idle(
        this: &LoopUtilsImpl,
        source: &mut Pin<Box<LoopUtilsSource>>,
        enabled: bool,
    ) -> std::io::Result<i32> {
        let loop_utils = Self::from_loop_utils(this);
        let funcs = loop_utils.iface.cb.funcs as *const CLoopUtilsMethods;
        let source = *source.inner.downcast_ref::<*mut CSource>().unwrap();

        result_from(unsafe { ((*funcs).enable_idle)(loop_utils.iface.cb.data, source, enabled) })
    }

    #[no_mangle]
    extern "C" fn source_event_trampoline(data: *mut c_void, count: c_ulong) {
        let source = unsafe { (data as *mut LoopUtilsSource).as_mut().unwrap() };

        let func = match source.cb {
            LoopUtilsSourceCb::Event(ref mut f) => f,
            _ => panic!("source_event_trampoline called with non-event source"),
        };

        (func)(count)
    }

    fn add_event(
        this: &LoopUtilsImpl,
        func: Box<SourceEventFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        let loop_utils = Self::from_loop_utils(this);
        let funcs = loop_utils.iface.cb.funcs as *const CLoopUtilsMethods;

        let mut source = Box::pin(LoopUtilsSource {
            cb: LoopUtilsSourceCb::Event(func),
            inner: Box::new(std::ptr::null_mut::<CSource>()),
        });

        let c_source = unsafe {
            ((*funcs).add_event)(
                loop_utils.iface.cb.data,
                Self::source_event_trampoline,
                &mut *source as *mut LoopUtilsSource as *mut c_void,
            )
        };

        if c_source.is_null() {
            None
        } else {
            source.inner = Box::new(c_source);
            Some(source)
        }
    }

    fn signal_event(
        this: &LoopUtilsImpl,
        source: &mut Pin<Box<LoopUtilsSource>>,
    ) -> std::io::Result<i32> {
        let loop_utils = Self::from_loop_utils(this);
        let funcs = loop_utils.iface.cb.funcs as *const CLoopUtilsMethods;
        let source = *source.inner.downcast_ref::<*mut CSource>().unwrap();

        result_from(unsafe { ((*funcs).signal_event)(loop_utils.iface.cb.data, source) })
    }

    #[no_mangle]
    extern "C" fn source_timer_trampoline(data: *mut c_void, expirations: c_ulong) {
        let source = unsafe { (data as *mut LoopUtilsSource).as_mut().unwrap() };

        let func = match source.cb {
            LoopUtilsSourceCb::Timer(ref mut f) => f,
            _ => panic!("source_timer_trampoline called with non-timer source"),
        };

        (func)(expirations)
    }

    fn add_timer(
        this: &LoopUtilsImpl,
        func: Box<SourceTimerFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        let loop_utils = Self::from_loop_utils(this);
        let funcs = loop_utils.iface.cb.funcs as *const CLoopUtilsMethods;

        let mut source = Box::pin(LoopUtilsSource {
            cb: LoopUtilsSourceCb::Timer(func),
            inner: Box::new(std::ptr::null_mut::<CSource>()),
        });

        let c_source = unsafe {
            ((*funcs).add_timer)(
                loop_utils.iface.cb.data,
                Self::source_timer_trampoline,
                &mut *source as *mut LoopUtilsSource as *mut c_void,
            )
        };

        if c_source.is_null() {
            None
        } else {
            source.inner = Box::new(c_source);
            Some(source)
        }
    }

    fn update_timer(
        this: &LoopUtilsImpl,
        source: &mut Pin<Box<LoopUtilsSource>>,
        value: &libc::timespec,
        interval: Option<&libc::timespec>,
        absolute: bool,
    ) -> std::io::Result<i32> {
        let loop_utils = Self::from_loop_utils(this);
        let funcs = loop_utils.iface.cb.funcs as *const CLoopUtilsMethods;
        let source = *source.inner.downcast_ref::<*mut CSource>().unwrap();

        let interval = match interval {
            Some(i) => i as &libc::timespec,
            None => std::ptr::null(),
        };

        result_from(unsafe {
            ((*funcs).update_timer)(loop_utils.iface.cb.data, source, value, interval, absolute)
        })
    }

    #[no_mangle]
    extern "C" fn source_signal_trampoline(data: *mut c_void, signal_number: c_int) {
        let source = unsafe { (data as *mut LoopUtilsSource).as_mut().unwrap() };

        let func = match source.cb {
            LoopUtilsSourceCb::Signal(ref mut f) => f,
            _ => panic!("source_signal_trampoline called with non-signal source"),
        };

        (func)(signal_number)
    }

    fn add_signal(
        this: &LoopUtilsImpl,
        signal_number: i32,
        func: Box<SourceSignalFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        let loop_utils = Self::from_loop_utils(this);
        let funcs = loop_utils.iface.cb.funcs as *const CLoopUtilsMethods;

        let mut source = Box::pin(LoopUtilsSource {
            cb: LoopUtilsSourceCb::Signal(func),
            inner: Box::new(std::ptr::null_mut::<CSource>()),
        });

        let c_source = unsafe {
            ((*funcs).add_signal)(
                loop_utils.iface.cb.data,
                signal_number,
                Self::source_signal_trampoline,
                &mut *source as *mut LoopUtilsSource as *mut c_void,
            )
        };

        if c_source.is_null() {
            None
        } else {
            source.inner = Box::new(c_source);
            Some(source)
        }
    }

    fn destroy_source(this: &LoopUtilsImpl, source: Pin<Box<LoopUtilsSource>>) {
        let loop_utils = Self::from_loop_utils(this);
        let funcs = loop_utils.iface.cb.funcs as *const CLoopUtilsMethods;
        let c_source = *source.inner.downcast_ref::<*mut CSource>().unwrap();

        unsafe {
            ((*funcs).destroy_source)(loop_utils.iface.cb.data, c_source);
        }
    }
}

struct LoopUtilsCIface {
    loop_utils: *const LoopUtilsImpl,
    sources: HashMap<*mut CSource, Pin<Box<LoopUtilsSource>>>,
}

impl LoopUtilsCIface {
    fn c_to_loop_utils_impl(object: *mut c_void) -> &'static mut Pin<Box<LoopUtilsCIface>> {
        unsafe { (object as *mut Pin<Box<LoopUtilsCIface>>).as_mut().unwrap() }
    }

    extern "C" fn add_io(
        object: *mut c_void,
        fd: c_int,
        mask: u32,
        close: bool,
        func: CSourceIoFn,
        data: *mut c_void,
    ) -> *mut CSource {
        let iface = Self::c_to_loop_utils_impl(object);
        let loop_utils = unsafe { iface.loop_utils.as_ref().unwrap() };

        let source = match loop_utils.add_io(
            fd,
            mask,
            close,
            Box::new(move |fd, mask| func(data, fd, mask)),
        ) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let c_source = *source.inner.downcast_ref::<*mut CSource>().unwrap();

        iface.sources.insert(c_source, source);

        c_source
    }

    extern "C" fn update_io(object: *mut c_void, c_source: *mut CSource, mask: u32) -> c_int {
        let iface = Self::c_to_loop_utils_impl(object);
        let loop_utils = unsafe { iface.loop_utils.as_ref().unwrap() };
        let source = iface.sources.get_mut(&c_source).unwrap();

        let res = loop_utils.update_io(source, mask);

        match res {
            Ok(_) => 0,
            Err(e) => -e.raw_os_error().unwrap_or(-1),
        }
    }

    extern "C" fn add_idle(
        object: *mut c_void,
        enabled: bool,
        func: CSourceIdleFn,
        data: *mut c_void,
    ) -> *mut CSource {
        let iface = Self::c_to_loop_utils_impl(object);
        let loop_utils = unsafe { iface.loop_utils.as_ref().unwrap() };

        let source = match loop_utils.add_idle(enabled, Box::new(move || func(data))) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let c_source = *source.inner.downcast_ref::<*mut CSource>().unwrap();

        iface.sources.insert(c_source, source);

        c_source
    }

    extern "C" fn enable_idle(object: *mut c_void, c_source: *mut CSource, enabled: bool) -> c_int {
        let iface = Self::c_to_loop_utils_impl(object);
        let loop_utils = unsafe { iface.loop_utils.as_ref().unwrap() };

        let source = iface.sources.get_mut(&c_source).unwrap();

        let res = loop_utils.enable_idle(source, enabled);

        match res {
            Ok(_) => 0,
            Err(e) => -e.raw_os_error().unwrap_or(-1),
        }
    }

    extern "C" fn add_event(
        object: *mut c_void,
        func: CSourceEventFn,
        data: *mut c_void,
    ) -> *mut CSource {
        let iface = Self::c_to_loop_utils_impl(object);
        let loop_utils = unsafe { iface.loop_utils.as_ref().unwrap() };

        let source = match loop_utils.add_event(Box::new(move |count| func(data, count))) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let c_source = *source.inner.downcast_ref::<*mut CSource>().unwrap();

        iface.sources.insert(c_source, source);

        c_source
    }

    extern "C" fn signal_event(object: *mut c_void, c_source: *mut CSource) -> c_int {
        let iface = Self::c_to_loop_utils_impl(object);
        let loop_utils = unsafe { iface.loop_utils.as_ref().unwrap() };

        let source = iface.sources.get_mut(&c_source).unwrap();

        let res = loop_utils.signal_event(source);

        match res {
            Ok(_) => 0,
            Err(e) => -e.raw_os_error().unwrap_or(-1),
        }
    }

    extern "C" fn add_timer(
        object: *mut c_void,
        func: CSourceTimerFn,
        data: *mut c_void,
    ) -> *mut CSource {
        let iface = Self::c_to_loop_utils_impl(object);
        let loop_utils = unsafe { iface.loop_utils.as_ref().unwrap() };

        let source = match loop_utils.add_event(Box::new(move |exp| func(data, exp))) {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let c_source = *source.inner.downcast_ref::<*mut CSource>().unwrap();

        iface.sources.insert(c_source, source);

        c_source
    }

    extern "C" fn update_timer(
        object: *mut c_void,
        c_source: *mut CSource,
        value: *const libc::timespec,
        interval: *const libc::timespec,
        absolute: bool,
    ) -> c_int {
        let iface = Self::c_to_loop_utils_impl(object);
        let loop_utils = unsafe { iface.loop_utils.as_ref().unwrap() };

        let source = iface.sources.get_mut(&c_source).unwrap();

        let interval = if interval.is_null() {
            None
        } else {
            unsafe { Some(&*interval) }
        };

        let res = loop_utils.update_timer(source, unsafe { &*value }, interval, absolute);

        match res {
            Ok(_) => 0,
            Err(e) => -e.raw_os_error().unwrap_or(-1),
        }
    }

    extern "C" fn add_signal(
        object: *mut c_void,
        signal_number: c_int,
        func: CSourceSignalFn,
        data: *mut c_void,
    ) -> *mut CSource {
        let iface = Self::c_to_loop_utils_impl(object);
        let loop_utils = unsafe { iface.loop_utils.as_ref().unwrap() };

        let source = match loop_utils
            .add_signal(signal_number, Box::new(move |signum| func(data, signum)))
        {
            Some(s) => s,
            None => return std::ptr::null_mut(),
        };
        let c_source = *source.inner.downcast_ref::<*mut CSource>().unwrap();

        iface.sources.insert(c_source, source);

        c_source
    }

    extern "C" fn destroy_source(object: *mut c_void, c_source: *mut CSource) {
        let iface = Self::c_to_loop_utils_impl(object);
        let loop_utils = unsafe { iface.loop_utils.as_ref().unwrap() };

        let source = iface.sources.remove(&c_source).unwrap();

        loop_utils.destroy_source(source)
    }
}

static LOOP_UTILS_METHODS: CLoopUtilsMethods = CLoopUtilsMethods {
    version: 0,

    add_io: LoopUtilsCIface::add_io,
    update_io: LoopUtilsCIface::update_io,
    add_idle: LoopUtilsCIface::add_idle,
    enable_idle: LoopUtilsCIface::enable_idle,
    add_event: LoopUtilsCIface::add_event,
    signal_event: LoopUtilsCIface::signal_event,
    add_timer: LoopUtilsCIface::add_timer,
    update_timer: LoopUtilsCIface::update_timer,
    add_signal: LoopUtilsCIface::add_signal,
    destroy_source: LoopUtilsCIface::destroy_source,
};

pub(crate) unsafe fn make_native(loop_utils: &LoopUtilsImpl) -> *mut CInterface {
    let c_loop_utils: *mut CLoopUtils = unsafe {
        libc::calloc(1, std::mem::size_of::<CLoopUtils>() as libc::size_t) as *mut CLoopUtils
    };

    let loop_utils_iface = Box::pin(LoopUtilsCIface {
        loop_utils: loop_utils as *const LoopUtilsImpl,
        sources: HashMap::new(),
    });

    let c_loop_utils = unsafe { &mut *c_loop_utils };

    c_loop_utils.iface.version = 0;
    c_loop_utils.iface.type_ = c_string(interface::CPU).into_raw();
    c_loop_utils.iface.cb.funcs = &LOOP_UTILS_METHODS as *const CLoopUtilsMethods as *mut c_void;
    c_loop_utils.iface.cb.data =
        &loop_utils_iface as *const Pin<Box<LoopUtilsCIface>> as *mut c_void;

    c_loop_utils as *mut CLoopUtils as *mut CInterface
}

pub(crate) unsafe fn free_native(c_loop_util: *mut CInterface) {
    unsafe {
        let _ = CString::from_raw((*c_loop_util).type_ as *mut i8);
        libc::free(c_loop_util as *mut c_void);
    }
}
