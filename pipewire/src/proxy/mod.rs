// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};

use pipewire_native_spa as spa;

use crate::{refcounted, Refcounted, INVALID_ID};

use crate::{types::ObjectType, Id};

pub mod client;

pub trait HasProxy {
    fn type_() -> ObjectType
    where
        Self: Sized;
    fn version() -> u32
    where
        Self: Sized;

    fn proxy(&self) -> Proxy<Self>
    where
        Self: Refcounted;
}

refcounted! {
    pub struct Proxy<T: Refcounted> {
        object: ProxyObject<T>,
        id: Id,
        bound_id: RefCell<Id>,
        hooks: Arc<Mutex<spa::hook::HookList<ProxyEvents>>>,
    }
}

// Allow storring a strong or weak reference inside the proxy. For client-side objects like the
// core, a weak reference suffices so we don't create reference cycles between the object and its
// proxy.
//
// For server-side objects, the proxy can hold a strong reference to the object as the owner.
enum ProxyObject<T: Refcounted> {
    Strong(T),
    Weak(T::WeakRef),
}

pub struct ProxyEvents {
    pub destroy: Option<Box<dyn FnMut()>>,
    pub bound: Option<Box<dyn FnMut(Id)>>,
    pub removed: Option<Box<dyn FnMut()>>,
    pub done: Option<Box<dyn FnMut(u32)>>,
    pub error: Option<Box<dyn FnMut(u32, u32, &str)>>,
    pub bound_props: Option<Box<dyn FnMut(u32, &spa::dict::Dict)>>,
}

impl<T: Refcounted> Proxy<T> {
    pub(crate) fn new(id: Id, object: &T) -> Self {
        Self {
            inner: Rc::new(InnerProxy::<T>::new(
                id,
                ProxyObject::Strong(object.clone()),
            )),
        }
    }

    pub(crate) fn new_weak(id: Id, object: &T) -> Self {
        Self {
            inner: Rc::new(InnerProxy::<T>::new(
                id,
                ProxyObject::Weak(object.downgrade()),
            )),
        }
    }

    pub fn object(&self) -> Option<T> {
        match &self.inner.object {
            ProxyObject::Strong(object) => Some(object.clone()),
            ProxyObject::Weak(object) => Refcounted::upgrade(object),
        }
    }

    pub(crate) fn set_bound_id(&self, id: Id) {
        self.inner.bound_id.replace(id);
        spa::emit_hook!(self.inner.hooks, bound, id);
    }
}

impl<T: Refcounted> InnerProxy<T> {
    fn new(id: Id, object: ProxyObject<T>) -> Self {
        Self {
            object,
            id,
            bound_id: RefCell::new(INVALID_ID),
            hooks: spa::hook::HookList::new(),
        }
    }
}
