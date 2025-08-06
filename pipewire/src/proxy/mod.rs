// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::any::Any;
use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::{Arc, Mutex};

use pipewire_native_spa as spa;

use crate::{properties::Properties, refcounted, Refcounted, INVALID_ID};

use crate::{types::ObjectType, Id};

pub mod client;

pub trait HasProxy: Any {
    // See the invoke! and notify! macros below
    // type Methods;
    // type Events;

    fn type_(&self) -> ObjectType;

    fn version() -> u32
    where
        Self: Sized;

    fn proxy(&self) -> Proxy<Self>
    where
        Self: Refcounted;
}

impl dyn HasProxy {
    pub fn downcast_proxy<T: HasProxy + Refcounted>(&self) -> Option<Proxy<T>> {
        if let Some(object) = (self as &dyn Any).downcast_ref::<T>() {
            Some(object.proxy())
        } else {
            None
        }
    }
}

// We expect each proxy object to have a set of associated methods (which can be invoked on the
// object) and/or events (which notify listeners via hooks). Unfortunately, expressing this via the
// type system makes things complicated. Notably, the `proxies` list on `Core` can no longer be a
// container of `dyn HasProxy`, as the associated types all need to be specified.
//
// As a compromise, we provide these two macros that assume types that implement `HasProxy` also
// implement either or both functions, methods() and events(). These return a struct of their
// respective types, on which an invocation or notification can be triggered.
#[macro_export]
macro_rules! proxy_object_invoke {
    ($proxy:ident, $method:ident, $($args:tt)*) => {
        ($proxy.object().unwrap().methods().borrow_mut().$method)(&$proxy, $($args)*)
    }
}

#[macro_export]
macro_rules! proxy_object_notify {
    ($proxy:ident, $event:ident, $($args:tt)*) => {
        if let Some(_object) = $proxy.object() {
            spa::emit_hook!(_object.events(), $event, $($args)*);
        }
    };
}

// To go from an object in dyn Proxy form to its proxy, we need to do some dyn Any shenanigans, so
// let's hide that away in a macro as well.
#[macro_export]
macro_rules! proxy_notify_dyn {
    ($object:ident, $event:ident, $($args:tt),*) => {
        let _type_id = ($object as &dyn std::any::Any).type_id();
        if _type_id ==  std::any::TypeId::of::<$crate::core::Core>() {
            let _proxy = $object.downcast_proxy::<$crate::core::Core>().unwrap();
            spa::emit_hook!(_proxy.events(), $event, $($args),*);
        } else if _type_id ==  std::any::TypeId::of::<$crate::proxy::client::Client>() {
            let _proxy = $object.downcast_proxy::<$crate::proxy::client::Client>().unwrap();
            spa::emit_hook!(_proxy.events(), $event, $($args),*);
        } else {
            unreachable!()
        };
    };
}

#[macro_export]
macro_rules! proxy_notify {
    ($object:ident, $event:ident, $($args:tt),*) => {
        spa::emit_hook!($object.proxy().events(), $event, $($args),*);
    };
}

refcounted! {
    pub struct Proxy<T: HasProxy + Refcounted> {
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
    pub bound_props: Option<Box<dyn FnMut(u32, &Properties)>>,
}

impl<T: HasProxy + Refcounted> Proxy<T> {
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

    pub fn id(&self) -> Id {
        self.inner.id
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

    pub(crate) fn add_listener(&self, events: ProxyEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    pub(crate) fn events(&self) -> Arc<Mutex<spa::hook::HookList<ProxyEvents>>> {
        self.inner.hooks.clone()
    }
}

impl<T: HasProxy + Refcounted> InnerProxy<T> {
    fn new(id: Id, object: ProxyObject<T>) -> Self {
        Self {
            object,
            id,
            bound_id: RefCell::new(INVALID_ID),
            hooks: spa::hook::HookList::new(),
        }
    }
}
