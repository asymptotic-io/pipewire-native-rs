// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{any::TypeId, pin::Pin, rc::Rc};

use crate::dict::Dict;

use super::ffi::CInterface;

pub const LOG_FACTORY: &str = "support.log";
pub const SYSTEM_FACTORY: &str = "support.system";
pub const CPU_FACTORY: &str = "support.cpu";
pub const LOOP_FACTORY: &str = "support.loop";

pub trait Interface {
    /// Return a C-compatible spa_interface pointer
    ///
    /// # Safety
    /// The caller must manually free the returned pointer using `free_native()`.
    unsafe fn make_native(&self) -> *mut CInterface;

    /// Return a C-compatible spa_interface pointer
    ///
    /// # Safety
    /// The pointer must have been allocated using `make_native()`.
    unsafe fn free_native(cpu: *mut CInterface)
    where
        Self: Sized;

    fn type_id(&self) -> TypeId
    where
        Self: 'static,
    {
        TypeId::of::<Self>()
    }
}

impl std::fmt::Debug for dyn Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interface")
            .field("type_id", &self.type_id())
            .finish()
    }
}

type RefcountedPinBox<T> = Rc<Pin<Box<T>>>;

impl dyn Interface {
    pub fn is<T>(&self) -> bool
    where
        T: 'static,
    {
        TypeId::of::<T>() == self.type_id()
    }

    pub fn downcast_box<T>(self: Box<Self>) -> Result<Box<T>, Box<Self>>
    where
        T: 'static,
    {
        if self.is::<T>() {
            Ok(unsafe { Box::from_raw(Box::into_raw(self) as *mut T) })
        } else {
            Err(self)
        }
    }

    pub fn downcast_rc_pin_box<T>(
        self: RefcountedPinBox<Self>,
    ) -> Result<RefcountedPinBox<T>, RefcountedPinBox<Self>>
    where
        T: 'static,
    {
        if self.is::<T>() {
            Ok(unsafe { Rc::from_raw(Rc::into_raw(self) as *mut Pin<Box<T>>) })
        } else {
            Err(self)
        }
    }
}

pub struct InterfaceInfo {
    pub type_: String,
}

pub trait HandleFactory {
    /* Data fields */
    fn version(&self) -> u32;
    fn name(&self) -> &str;
    fn info(&self) -> Option<&Dict>;

    /* Methods */
    fn init(
        &self,
        info: Option<Dict>,
        support: &super::Support,
    ) -> std::io::Result<Box<dyn Handle + Send + Sync>>;
    fn enum_interface_info(&self) -> Vec<InterfaceInfo>;
}

pub trait Handle {
    /* Data fields */
    fn version(&self) -> u32;

    /* Methods */
    /* FIXME: The interface implicitly depends on the Handle, and we likely should express that as
     * a lifetime dependency between the Handle and the Interface. We could also return a
     * reference, but then the caller might have to perform some shenanigans to keep both the
     * handle and a reference to something from the handle around. */
    fn get_interface(&self, type_: &str) -> Option<Box<dyn Interface>>;
}
