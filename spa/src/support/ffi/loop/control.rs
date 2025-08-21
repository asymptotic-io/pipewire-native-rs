// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Sanchayan Maity

use std::cell::RefCell;
use std::ffi::{c_int, c_uint, c_void, CString};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::emit_hook;
use crate::hook::{HookId, HookList};
use crate::interface::ffi::{CControlHooks, CHook};
use crate::interface::r#loop::*;
use crate::interface::{self, ffi::CInterface};
use crate::support::ffi::c_string;
use crate::support::ffi::r#loop::common::{from_result, result_from};

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CLoopControlMethods {
    version: u32,

    get_fd: extern "C" fn(object: *mut c_void) -> c_uint,
    add_hook: extern "C" fn(
        object: *mut c_void,
        hook: *mut CHook,
        hooks: *const CControlHooks,
        data: *mut c_void,
    ),
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
struct CLoopControlIface {
    iface: CInterface,
}

#[repr(C)]
struct CLoopControlImpl {
    iface: *mut CLoopControlIface,
    hooks: Arc<Mutex<HookList<LoopControlHooks>>>,
    c_hook: CHook,
    c_hook_methods: CControlHooks,
}

extern "C" fn loop_before_trampoline(data: *mut c_void) {
    let loop_control_impl_ = unsafe { (data as *mut CLoopControlImpl).as_ref().unwrap() };

    emit_hook!(loop_control_impl_.hooks, before);
}

extern "C" fn loop_after_trampoline(data: *mut c_void) {
    let loop_control_impl_ = unsafe { (data as *mut CLoopControlImpl).as_ref().unwrap() };

    emit_hook!(loop_control_impl_.hooks, after);
}

pub fn new_impl(interface: *mut CInterface) -> LoopControlImpl {
    let inner = Box::pin(Rc::new(RefCell::new(CLoopControlImpl {
        iface: interface as *mut CLoopControlIface,
        hooks: HookList::new(),
        c_hook: CHook::new_uninit(),
        c_hook_methods: CControlHooks {
            version: 0,
            before: loop_before_trampoline,
            after: loop_after_trampoline,
        },
    })));

    let cb_data = inner.borrow().methods().iface.cb.data;
    let funcs = inner.borrow().methods().iface.cb.funcs as *const CLoopControlMethods;

    let mut temp_inner = inner.borrow_mut();
    let c_hook = (&mut temp_inner.c_hook) as *mut CHook;
    let c_hook_methods = &temp_inner.c_hook_methods as *const CControlHooks;
    // The struct is repr(C), so get a pointer to the first member and use that to cast to the
    // structure itself. FIXME: using the Pin<Box<Rc<RefCell<...>>>> caused a crash in the
    // trampoline, but we should use that if we can.
    let user_data = &mut temp_inner.iface as *mut *mut CLoopControlIface as *mut c_void;

    unsafe {
        ((*funcs).add_hook)(cb_data, c_hook, c_hook_methods, user_data);
    }

    drop(temp_inner);

    LoopControlImpl {
        inner,

        get_fd: CLoopControlImpl::get_fd,
        add_hook: CLoopControlImpl::add_hook,
        remove_hook: CLoopControlImpl::remove_hook,
        enter: CLoopControlImpl::enter,
        leave: CLoopControlImpl::leave,
        iterate: CLoopControlImpl::iterate,
        check: CLoopControlImpl::check,
        lock: CLoopControlImpl::lock,
        unlock: CLoopControlImpl::unlock,
        get_time: CLoopControlImpl::get_time,
        wait: CLoopControlImpl::wait,
        signal: CLoopControlImpl::signal,
        accept: CLoopControlImpl::accept,
    }
}

impl CLoopControlImpl {
    fn from_control(this: &LoopControlImpl) -> Rc<RefCell<CLoopControlImpl>> {
        this.inner
            .as_ref()
            .downcast_ref::<Rc<RefCell<CLoopControlImpl>>>()
            .unwrap()
            .clone()
    }

    fn methods(&self) -> &CLoopControlIface {
        unsafe { self.iface.as_ref().unwrap() }
    }

    fn get_fd(this: &LoopControlImpl) -> u32 {
        let impl_rc = Self::from_control(this);
        let control_impl = impl_rc.borrow();
        let methods = control_impl.methods();

        let funcs = methods.iface.cb.funcs as *const CLoopControlMethods;

        unsafe { ((*funcs).get_fd)(methods.iface.cb.data) }
    }

    fn add_hook(this: &LoopControlImpl, hooks: LoopControlHooks) -> HookId {
        let control_impl = Self::from_control(this);

        let id = control_impl
            .borrow_mut()
            .hooks
            .lock()
            .unwrap()
            .append(hooks);

        id
    }

    fn remove_hook(this: &LoopControlImpl, hook: HookId) {
        let control_impl = Self::from_control(this);

        control_impl.borrow_mut().hooks.lock().unwrap().remove(hook);
    }

    fn enter(this: &LoopControlImpl) {
        let impl_rc = Self::from_control(this);
        let control_impl = impl_rc.borrow();
        let methods = control_impl.methods();

        let funcs = methods.iface.cb.funcs as *const CLoopControlMethods;

        unsafe { ((*funcs).enter)(methods.iface.cb.data) }
    }

    fn leave(this: &LoopControlImpl) {
        let impl_rc = Self::from_control(this);
        let control_impl = impl_rc.borrow();
        let methods = control_impl.methods();

        let funcs = methods.iface.cb.funcs as *const CLoopControlMethods;

        unsafe {
            ((*funcs).leave)(methods.iface.cb.data);
        }
    }

    fn iterate(this: &LoopControlImpl, timeout: Option<Duration>) -> std::io::Result<i32> {
        let impl_rc = Self::from_control(this);
        let control_impl = impl_rc.borrow();
        let methods = control_impl.methods();

        let funcs = methods.iface.cb.funcs as *const CLoopControlMethods;

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

        result_from(unsafe { ((*funcs).iterate)(methods.iface.cb.data, timeout) })
    }

    fn check(this: &LoopControlImpl) -> std::io::Result<i32> {
        let impl_rc = Self::from_control(this);
        let control_impl = impl_rc.borrow();
        let methods = control_impl.methods();

        let funcs = methods.iface.cb.funcs as *const CLoopControlMethods;

        result_from(unsafe { ((*funcs).check)(methods.iface.cb.data) })
    }

    fn lock(this: &LoopControlImpl) -> std::io::Result<i32> {
        let impl_rc = Self::from_control(this);
        let control_impl = impl_rc.borrow();
        let methods = control_impl.methods();

        let funcs = methods.iface.cb.funcs as *const CLoopControlMethods;

        unsafe {
            #[allow(clippy::cmp_null)]
            if (*funcs).lock as *const c_void != std::ptr::null() {
                result_from(((*funcs).lock)(methods.iface.cb.data))
            } else {
                Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
            }
        }
    }

    fn unlock(this: &LoopControlImpl) -> std::io::Result<i32> {
        let impl_rc = Self::from_control(this);
        let control_impl = impl_rc.borrow();
        let methods = control_impl.methods();

        let funcs = methods.iface.cb.funcs as *const CLoopControlMethods;

        unsafe {
            #[allow(clippy::cmp_null)]
            if (*funcs).lock as *const c_void != std::ptr::null() {
                result_from(((*funcs).unlock)(methods.iface.cb.data))
            } else {
                Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
            }
        }
    }

    fn get_time(this: &LoopControlImpl, timeout: Duration) -> std::io::Result<libc::timespec> {
        let mut abstime = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        let impl_rc = Self::from_control(this);
        let control_impl = impl_rc.borrow();
        let methods = control_impl.methods();

        let funcs = methods.iface.cb.funcs as *const CLoopControlMethods;

        let res = unsafe {
            ((*funcs).get_time)(
                methods.iface.cb.data,
                &mut abstime as *mut libc::timespec,
                timeout.as_nanos() as i64,
            )
        };

        match res {
            0 => Ok(abstime),
            e => Err(std::io::Error::from_raw_os_error(-e)),
        }
    }

    fn wait(this: &LoopControlImpl, abstime: &libc::timespec) -> std::io::Result<i32> {
        let impl_rc = Self::from_control(this);
        let control_impl = impl_rc.borrow();
        let methods = control_impl.methods();

        let funcs = methods.iface.cb.funcs as *const CLoopControlMethods;

        result_from(unsafe {
            ((*funcs).wait)(methods.iface.cb.data, abstime as *const libc::timespec)
        })
    }

    fn signal(this: &LoopControlImpl, wait_for_accept: bool) -> std::io::Result<i32> {
        let impl_rc = Self::from_control(this);
        let control_impl = impl_rc.borrow();
        let methods = control_impl.methods();

        let funcs = methods.iface.cb.funcs as *const CLoopControlMethods;

        result_from(unsafe { ((*funcs).signal)(methods.iface.cb.data, wait_for_accept) })
    }

    fn accept(this: &LoopControlImpl) -> std::io::Result<i32> {
        let impl_rc = Self::from_control(this);
        let control_impl = impl_rc.borrow();
        let methods = control_impl.methods();

        let funcs = methods.iface.cb.funcs as *const CLoopControlMethods;

        result_from(unsafe { ((*funcs).accept)(methods.iface.cb.data) })
    }
}

static LOOP_CONTROL_METHODS: CLoopControlMethods = CLoopControlMethods {
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

struct ControlHookPriv {
    impl_: &'static LoopControlImpl,
    id: HookId,
}

impl ControlMethodsIface {
    fn c_to_control_methods_impl(object: *mut c_void) -> &'static LoopControlImpl {
        unsafe { &*(object as *mut LoopControlImpl) }
    }

    extern "C" fn get_fd(object: *mut c_void) -> c_uint {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        control_methods_impl.get_fd()
    }

    extern "C" fn hook_removed(hook: *mut CHook) {
        let hook = unsafe { hook.as_mut().unwrap() };
        let priv_ = unsafe { Box::from_raw(hook.priv_ as *mut ControlHookPriv) };

        priv_.impl_.remove_hook(priv_.id);
    }

    extern "C" fn add_hook(
        object: *mut c_void,
        hook: *mut CHook,
        hooks: *const CControlHooks,
        data: *mut c_void,
    ) {
        let control_methods_impl = Self::c_to_control_methods_impl(object);

        let hooks = unsafe { hooks.as_ref().unwrap() };
        let before = hooks.before;
        let after = hooks.after;

        let id = control_methods_impl.add_hook(LoopControlHooks {
            before: Some(Box::new(move || {
                #[allow(clippy::cmp_null)]
                if before as *const c_void != std::ptr::null() {
                    (before)(data);
                }
            })),
            after: Some(Box::new(move || {
                #[allow(clippy::cmp_null)]
                if after as *const c_void != std::ptr::null() {
                    (after)(data);
                }
            })),
        });

        // On the C side, hook removal just happens by the owner of the hook calling
        // spa_hook_remove(), and the hook list owner is not supposed to care. In our case, we are
        // doing a translation of C hooks to Rust hooks, so we need to care. To manage this, we set
        // up the removed callback to notify us, and do the cleanup there.
        let hook = unsafe { hook.as_mut().unwrap() };

        hook.removed = Self::hook_removed;
        hook.priv_ = Box::into_raw(Box::new(ControlHookPriv {
            impl_: control_methods_impl,
            id,
        })) as *mut c_void;
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

pub(crate) unsafe fn make_native(loop_ctrl: &LoopControlImpl) -> *mut CInterface {
    let c_ctrl_methods: *mut CLoopControlIface = unsafe {
        libc::calloc(1, std::mem::size_of::<CLoopControlIface>() as libc::size_t)
            as *mut CLoopControlIface
    };
    let c_ctrl_methods = unsafe { &mut *c_ctrl_methods };

    c_ctrl_methods.iface.version = 1;
    c_ctrl_methods.iface.type_ = c_string(interface::CPU).into_raw();
    c_ctrl_methods.iface.cb.funcs =
        &LOOP_CONTROL_METHODS as *const CLoopControlMethods as *mut c_void;
    c_ctrl_methods.iface.cb.data = loop_ctrl as *const LoopControlImpl as *mut c_void;

    c_ctrl_methods as *mut CLoopControlIface as *mut CInterface
}

pub(crate) unsafe fn free_native(c_loop_ctrl: *mut CInterface) {
    unsafe {
        let _ = CString::from_raw((*c_loop_ctrl).type_ as *mut i8);
        libc::free(c_loop_ctrl as *mut c_void);
    }
}
