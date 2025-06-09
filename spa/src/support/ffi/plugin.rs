// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::ffi::{c_char, c_int, c_void, CStr};
use std::path::PathBuf;

use libloading::os::unix::{Library, Symbol, RTLD_NOW};

use crate::dict::Dict;
use crate::interface::ffi::{CInterface, CSupport};
use crate::interface::plugin::{Handle, HandleFactory, Interface, InterfaceInfo};
use crate::interface::{self, Support};

use super::cpu;
use super::log;
use super::system;
use super::{c_string, r#loop};

const ENTRYPOINT: &str = "spa_handle_factory_enum";
type EntryPointFn = unsafe extern "C" fn(*const *mut CHandleFactory, *mut u32) -> c_int;

pub struct Plugin {
    _library: Library,
    factories: Vec<*mut CHandleFactory>,
}

impl Plugin {
    pub fn find_factory(&self, name: &str) -> Option<Box<dyn HandleFactory>> {
        for f in &self.factories {
            let f_name = unsafe {
                let factory = f.as_ref().unwrap();
                CStr::from_ptr(factory.name).to_str()
            };

            if f_name == Ok(name) {
                return Some(Box::new(CHandleFactoryImpl { factory: *f }));
            }
        }

        None
    }
}

#[repr(C)]
#[derive(Copy)]
pub struct CInterfaceInfo {
    pub type_: *const c_char,
}

impl Clone for CInterfaceInfo {
    fn clone(&self) -> Self {
        /*
         * Making the implementation be like below, would be a non-canonical
         * implementation of Clone since Copy is already implemented.
         * See https://rust-lang.github.io/rust-clippy/master/index.html#non_canonical_clone_impl

           CInterfaceInfo {
               type_: unsafe { libc::strdup(self.type_) },
           }
        */
        *self
    }
}

#[repr(C)]
pub struct CHandleFactory {
    pub version: u32,
    pub name: *const c_char,
    pub info: *const Dict,

    pub get_size: fn(factory: *const CHandleFactory, params: *const Dict) -> usize,
    pub init: fn(
        factory: *const CHandleFactory,
        handle: *mut CHandle,
        params: *const Dict,
        support: *const CSupport,
        n_support: u32,
    ) -> c_int,
    pub enum_interface_info: fn(
        factory: *const CHandleFactory,
        info: *const *mut CInterfaceInfo,
        index: *mut u32,
    ) -> c_int,
}

struct CHandleFactoryImpl {
    factory: *mut CHandleFactory,
}

impl HandleFactory for CHandleFactoryImpl {
    fn version(&self) -> u32 {
        unsafe { self.factory.as_ref().unwrap().version }
    }

    fn name(&self) -> &str {
        unsafe {
            CStr::from_ptr(self.factory.as_ref().unwrap().name)
                .to_str()
                .unwrap()
        }
    }

    fn info(&self) -> Option<&Dict> {
        unsafe { self.factory.as_ref().unwrap().info.as_ref() }
    }

    fn init(&self, info: Option<Dict>, support: &Support) -> std::io::Result<Box<dyn Handle + Send + Sync>> {
        unsafe {
            let info_ptr = match &info {
                Some(i) => i.as_raw(),
                None => std::ptr::null(),
            };
            let size = (self.factory.as_ref().unwrap().get_size)(self.factory, info_ptr);
            let handle = libc::malloc(size) as *mut CHandle;
            let (support, n_support) = {
                let c_support = support.c_support();
                (c_support.as_ptr(), c_support.len())
            };
            let ret = (self.factory.as_ref().unwrap().init)(
                self.factory,
                handle,
                info_ptr,
                support,
                n_support as u32,
            );

            match ret {
                0 => Ok(Box::new(CHandleImpl { handle })),
                err => Err(std::io::Error::from_raw_os_error(err)),
            }
        }
    }

    fn enum_interface_info(&self) -> Vec<crate::interface::plugin::InterfaceInfo> {
        let mut interfaces = vec![];
        let info: *mut CInterfaceInfo = std::ptr::null_mut();
        let mut i: u32 = 0;

        loop {
            unsafe {
                match (self.factory.as_ref().unwrap().enum_interface_info)(
                    self.factory,
                    &info,
                    &mut i,
                ) {
                    1 => interfaces.push(InterfaceInfo {
                        type_: CStr::from_ptr((*info).type_).to_string_lossy().to_string(),
                    }),
                    _ => return interfaces,
                }
            }
        }
    }
}

#[repr(C)]
pub struct CHandle {
    pub version: u32,
    pub get_interface:
        fn(handle: *const CHandle, type_: *const c_char, iface: *mut *mut CInterface) -> c_int,
    pub clear: fn(handle: *mut CHandle) -> c_int,
}

struct CHandleImpl {
    handle: *mut CHandle,
}

unsafe impl Send for CHandleImpl {}
unsafe impl Sync for CHandleImpl {}

impl Drop for CHandleImpl {
    fn drop(&mut self) {
        unsafe {
            (self.handle.as_ref().unwrap().clear)(self.handle);
            libc::free(self.handle as *mut c_void);
        }
    }
}

impl Handle for CHandleImpl {
    fn version(&self) -> u32 {
        unsafe { self.handle.as_ref().unwrap().version }
    }

    fn get_interface(&self, type_: &str) -> Option<Box<dyn Interface>> {
        let mut iface: *mut CInterface = std::ptr::null_mut();

        unsafe {
            (self.handle.as_ref().unwrap().get_interface)(
                self.handle,
                c_string(type_).as_ptr(),
                &mut iface,
            )
        };

        if iface.is_null() {
            return None;
        }

        match type_ {
            interface::CPU => Some(Box::new(cpu::new_impl(iface))),
            interface::LOG => Some(Box::new(log::new_impl(iface))),
            interface::LOOP => Some(Box::new(r#loop::new_impl(iface))),
            interface::LOOP_CONTROL => Some(Box::new(r#loop::control::new_impl(iface))),
            interface::LOOP_UTILS => Some(Box::new(r#loop::utils::new_impl(iface))),
            interface::SYSTEM => Some(Box::new(system::new_impl(iface))),
            _ => None,
        }
    }
}

pub fn load(path: &PathBuf) -> Result<Plugin, String> {
    unsafe {
        let library = Library::open(Some(path), RTLD_NOW).map_err(|e| format!("{}", e))?;
        let entrypoint: Symbol<EntryPointFn> = library
            .get(ENTRYPOINT.as_bytes())
            .map_err(|e| format!("{}", e))?;

        let h: *mut CHandleFactory = std::ptr::null_mut();
        let h_ptr: *const *mut CHandleFactory = &h;
        let mut i: u32 = 0;
        let i_ptr: *mut u32 = &mut i;
        let mut factories = vec![];

        loop {
            match entrypoint(h_ptr, i_ptr) {
                1 => factories.push(h),
                0 => break,
                err => return Err(format!("Could not load plugin: {}", err)),
            }
        }

        Ok(Plugin {
            _library: library,
            factories,
        })
    }
}
