// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    collections::HashMap,
    ffi::{c_int, c_void, CString},
    os::fd::RawFd,
    pin::Pin,
    sync::{Arc, RwLock},
};

use crate::interface::ffi::{CLoop, CSource};
use crate::interface::{
    self,
    ffi::CInterface,
    r#loop::{InvokeFn, LoopImpl, Source, SourceFn},
};

use crate::support::ffi::c_string;
use crate::support::ffi::r#loop::common::result_from;

pub mod common;
pub mod control;
pub mod utils;

struct SendRawPointer(*const c_void);
struct SendRawPointerMut(*mut c_void);

unsafe impl Send for SendRawPointer {}
unsafe impl Send for SendRawPointerMut {}

type CInvokeFunc = extern "C" fn(
    loop_: *const CLoop,
    async_: bool,
    seq: u32,
    data: *const c_void,
    size: libc::size_t,
    user_data: *mut c_void,
) -> c_int;

#[repr(C)]
struct CLoopMethods {
    version: u32,
    add_source: extern "C" fn(object: *mut c_void, source: *mut CSource) -> c_int,
    update_source: extern "C" fn(object: *mut c_void, source: *mut CSource) -> c_int,
    remove_source: extern "C" fn(object: *mut c_void, source: *mut CSource) -> c_int,
    invoke: extern "C" fn(
        object: *const c_void,
        func: CInvokeFunc,
        seq: u32,
        data: *const c_void,
        size: libc::size_t,
        block: bool,
        user_data: *mut c_void,
    ) -> c_int,
}

struct CLoopImpl {
    iface: *mut CLoop,
    sources: HashMap<RawFd, Pin<Box<CSourceImpl>>>,
}

// The only usage of CLoopImpl should be inside the inner below
unsafe impl Send for CLoopImpl {}
unsafe impl Sync for CLoopImpl {}

pub fn new_impl(interface: *mut CInterface) -> LoopImpl {
    LoopImpl {
        // Arc so we can take a reference out of the pinned box, and RwLock so we can mutate it
        inner: Box::pin(Arc::new(RwLock::new(CLoopImpl {
            iface: interface as *mut CLoop,
            sources: HashMap::new(),
        }))),

        add_source: CLoopImpl::add_source,
        update_source: CLoopImpl::update_source,
        remove_source: CLoopImpl::remove_source,
        invoke: CLoopImpl::invoke,
    }
}

#[repr(C)]
struct CSourceImpl {
    c_source: CSource,
    func: Box<SourceFn>,
}

impl CLoopImpl {
    fn from_loop(this: &LoopImpl) -> (Arc<RwLock<CLoopImpl>>, &CLoop) {
        let c_loopimpl = this
            .inner
            .as_ref()
            .downcast_ref::<Arc<RwLock<CLoopImpl>>>()
            .unwrap()
            .clone();
        let c_loop = unsafe { c_loopimpl.read().unwrap().iface.as_ref().unwrap() };

        (c_loopimpl, c_loop)
    }

    #[no_mangle]
    extern "C" fn source_trampoline(c_source: *mut CSource) {
        let source_impl = unsafe { (c_source as *mut CSourceImpl).as_mut().unwrap() };
        let source = Source {
            fd: source_impl.c_source.fd,
            mask: source_impl.c_source.mask,
            rmask: source_impl.c_source.rmask,
        };

        (source_impl.func)(&source);
    }

    fn add_source(
        loop_: &mut LoopImpl,
        source: &Source,
        func: Box<SourceFn>,
    ) -> std::io::Result<i32> {
        let (c_loopimpl, c_loop) = Self::from_loop(loop_);
        let funcs = unsafe {
            (c_loop.iface.cb.funcs as *const CLoopMethods)
                .as_ref()
                .unwrap()
        };

        let mut source_impl = Box::pin(CSourceImpl {
            c_source: CSource {
                loop_: c_loop,
                func: Self::source_trampoline,
                data: std::ptr::null_mut(), /* see below */
                fd: source.fd,
                mask: source.mask,
                rmask: source.rmask,
                priv_: std::ptr::null_mut(),
            },
            func,
        });

        let c_source = &mut source_impl.c_source as *mut CSource;

        c_loopimpl
            .write()
            .unwrap()
            .sources
            .insert(source_impl.c_source.fd, source_impl);

        result_from((funcs.add_source)(c_loop.iface.cb.data, c_source))
    }

    fn update_source(loop_: &mut LoopImpl, source: &Source) -> std::io::Result<i32> {
        let (c_loopimpl, c_loop) = Self::from_loop(loop_);
        let funcs = unsafe {
            (c_loop.iface.cb.funcs as *const CLoopMethods)
                .as_ref()
                .unwrap()
        };

        let mut loopimpl = c_loopimpl.write().unwrap();
        let source_impl = loopimpl
            .sources
            .get_mut(&source.fd)
            .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?;

        source_impl.c_source.mask = source.mask;

        result_from((funcs.update_source)(
            c_loop.iface.cb.data,
            &mut source_impl.c_source as *mut CSource,
        ))
    }

    fn remove_source(loop_: &mut LoopImpl, fd: RawFd) -> std::io::Result<i32> {
        let (c_loopimpl, c_loop) = Self::from_loop(loop_);
        let funcs = unsafe {
            (c_loop.iface.cb.funcs as *const CLoopMethods)
                .as_ref()
                .unwrap()
        };

        let mut source_impl = c_loopimpl
            .write()
            .unwrap()
            .sources
            .remove(&fd)
            .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?;

        result_from((funcs.remove_source)(
            c_loop.iface.cb.data,
            &mut source_impl.c_source as *mut CSource,
        ))
    }

    #[no_mangle]
    extern "C" fn invoke_trampoline(
        _loop: *const CLoop,
        async_: bool,
        seq: u32,
        data: *const c_void,
        size: libc::size_t,
        user_data: *mut c_void,
    ) -> i32 {
        //pub invoke: fn(&mut LoopImpl, func: Pin<Box<InvokeFn>>, block: bool) -> std::io::Result<i32>,
        //pub type InvokeFn = dyn FnMut(LoopImpl, bool, u32, &[u8]) -> i32 + 'static;
        let mut func = unsafe { Box::from_raw(user_data as *mut Box<InvokeFn>) };
        let data = unsafe { std::slice::from_raw_parts(data as *const u8, size) };

        (func)(async_, seq, data)
    }

    fn invoke(
        loop_: &LoopImpl,
        seq: u32,
        data: &[u8],
        block: bool,
        func: Box<InvokeFn>,
    ) -> std::io::Result<i32> {
        let (_, c_loop) = Self::from_loop(loop_);
        let funcs = unsafe {
            (c_loop.iface.cb.funcs as *const CLoopMethods)
                .as_ref()
                .unwrap()
        };
        // Double box to get a thin pointer to a Box<dyn ...>
        let invoke_func = Box::new(func);

        result_from((funcs.invoke)(
            c_loop.iface.cb.data,
            Self::invoke_trampoline,
            seq,
            data.as_ptr() as *const c_void,
            data.len() as libc::size_t,
            block,
            Box::into_raw(invoke_func) as *mut c_void,
        ))
    }
}

static LOOP_METHODS: CLoopMethods = CLoopMethods {
    version: 0,
    add_source: LoopImplIface::add_source,
    update_source: LoopImplIface::update_source,
    remove_source: LoopImplIface::remove_source,
    invoke: LoopImplIface::invoke,
};

pub(crate) unsafe fn make_native(loop_: &LoopImpl) -> *mut CInterface {
    let c_loop: *mut CLoop =
        unsafe { libc::calloc(1, std::mem::size_of::<CLoop>() as libc::size_t) as *mut CLoop };
    let c_loop = unsafe { &mut *c_loop };

    c_loop.iface.version = 0;
    c_loop.iface.type_ = c_string(interface::LOOP).into_raw();
    c_loop.iface.cb.funcs = &LOOP_METHODS as *const CLoopMethods as *mut c_void;
    c_loop.iface.cb.data = loop_ as *const LoopImpl as *mut c_void;

    c_loop as *mut CLoop as *mut CInterface
}

pub(crate) unsafe fn free_native(c_loop: *mut CInterface) {
    unsafe {
        let _ = CString::from_raw((*c_loop).type_ as *mut i8);
        libc::free(c_loop as *mut c_void);
    }
}

struct LoopImplIface {}

impl LoopImplIface {
    fn c_to_loop_impl(object: *const c_void) -> &'static LoopImpl {
        unsafe { (object as *const LoopImpl).as_ref().unwrap() }
    }

    fn c_to_loop_impl_mut(object: *mut c_void) -> &'static mut LoopImpl {
        unsafe { (object as *mut LoopImpl).as_mut().unwrap() }
    }

    extern "C" fn add_source(object: *mut c_void, source: *mut CSource) -> c_int {
        let loop_impl = Self::c_to_loop_impl_mut(object);
        let c_source = unsafe { source.as_mut().unwrap() };
        let impl_source = Source {
            fd: c_source.fd,
            mask: c_source.mask,
            rmask: c_source.rmask,
        };

        let res = loop_impl.add_source(&impl_source, Box::new(|_| (c_source.func)(c_source)));

        match res {
            Ok(_) => 0,
            Err(e) => -e.raw_os_error().unwrap_or(-1),
        }
    }

    extern "C" fn update_source(object: *mut c_void, source: *mut CSource) -> c_int {
        let loop_impl = Self::c_to_loop_impl_mut(object);
        let c_source = unsafe { source.as_mut().unwrap() };
        let impl_source = Source {
            fd: c_source.fd,
            mask: c_source.mask,
            rmask: c_source.rmask,
        };

        let res = loop_impl.update_source(&impl_source);

        match res {
            Ok(_) => 0,
            Err(e) => -e.raw_os_error().unwrap_or(-1),
        }
    }

    extern "C" fn remove_source(object: *mut c_void, source: *mut CSource) -> c_int {
        let loop_impl = Self::c_to_loop_impl_mut(object);
        let c_source = unsafe { source.as_mut().unwrap() };

        let res = loop_impl.remove_source(c_source.fd);

        match res {
            Ok(_) => 0,
            Err(e) => -e.raw_os_error().unwrap_or(-1),
        }
    }

    extern "C" fn invoke(
        object: *const c_void,
        func: CInvokeFunc,
        seq: u32,
        data: *const c_void,
        size: libc::size_t,
        block: bool,
        user_data: *mut c_void,
    ) -> c_int {
        let loop_impl = Self::c_to_loop_impl(object);
        let object = SendRawPointer(object);
        let user_data = SendRawPointerMut(user_data);

        let res = loop_impl.invoke(
            seq,
            unsafe { std::slice::from_raw_parts(data as *const u8, size) },
            block,
            Box::new(move |async_, seq, data| {
                // Shenanigans to let the closure type be Send
                let object = &object;
                let user_data = &user_data;

                func(
                    object.0 as *mut CLoop,
                    async_,
                    seq,
                    data.as_ptr() as *const c_void,
                    data.len(),
                    user_data.0,
                )
            }),
        );

        match res {
            Ok(_) => 0,
            Err(e) => -e.raw_os_error().unwrap_or(-1),
        }
    }
}
