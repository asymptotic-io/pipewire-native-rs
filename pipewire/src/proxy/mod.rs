// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use pipewire_native_spa as spa;

use crate::{properties::Properties, refcounted, Refcounted, INVALID_ID};

use crate::{types::ObjectType, Id};

pub mod client;
pub mod registry;

pub trait HasProxy: Any {
    // See the invoke! and notify! macros below
    // type Methods;
    // type Events;

    fn type_(&self) -> ObjectType;

    fn version(&self) -> u32;

    fn proxy(&self) -> Proxy<Self>
    where
        Self: Refcounted;
}

impl dyn HasProxy {
    pub fn downcast<T: HasProxy + Refcounted>(&self) -> Option<T> {
        if let Some(object) = (self as &dyn Any).downcast_ref::<T>() {
            Some(object.clone())
        } else {
            None
        }
    }

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
    ($proxy:ident, $method:ident $(, $($args:tt)*)?) => {
        ($proxy.object().unwrap().methods().borrow_mut().$method)(&$proxy $(, $($args)*)?)
    };
}

#[macro_export]
macro_rules! proxy_object_notify {
    ($proxy:ident, $event:ident $(, $($args:tt)*)?) => {
        if let Some(_object) = $proxy.object() {
            spa::emit_hook!(_object.events(), $event $(, $($args)*)?);
        }
    };
}

// To go from an object in dyn HasProxy form to the actual proxy itself, we need to do some dyn Any
// shenanigans, so let's hide that away in a macro as well.
#[macro_export]
macro_rules! hasproxy_method_call {
    ($object:expr, $method:ident $(, $($args:tt),*)?) => {
        {
            if $object.type_() == $crate::types::interface::CORE {
                let _proxy = $object.downcast_proxy::<$crate::core::Core>().unwrap();
                _proxy.$method($($($args),*)?)
            } else if $object.type_() == $crate::types::interface::CLIENT {
                let _proxy = $object.downcast_proxy::<$crate::proxy::client::Client>().unwrap();
                _proxy.$method($($($args),*)?)
            } else if $object.type_() == $crate::types::interface::REGISTRY {
                let _proxy = $object.downcast_proxy::<$crate::proxy::registry::Registry>().unwrap();
                _proxy.$method($($($args),*)?)
            } else {
                unreachable!("got unexpected proxy type {}", $object.type_())
            }
        }
    };
}

#[macro_export]
macro_rules! hasproxy_notify {
    ($object:ident, $event:ident $(, $($args:tt),*)?) => {
        if $object.type_() == $crate::types::interface::CORE {
            let _proxy = $object.downcast_proxy::<$crate::core::Core>().unwrap();
            spa::emit_hook!(_proxy.events(), $event $(, $($args),*)?)
        } else if $object.type_() == $crate::types::interface::CLIENT {
            let _proxy = $object.downcast_proxy::<$crate::proxy::client::Client>().unwrap();
            spa::emit_hook!(_proxy.events(), $event $(, $($args),*)?)
        } else if $object.type_() == $crate::types::interface::REGISTRY {
            let _proxy = $object.downcast_proxy::<$crate::proxy::registry::Registry>().unwrap();
            spa::emit_hook!(_proxy.events(), $event $(, $($args),*)?)
        } else {
            unreachable!("got unexpected proxy type {}", $object.type_())
        }
    };
}

#[macro_export]
macro_rules! proxy_notify {
    ($object:ident, $event:ident $(, $($args:tt),*)?) => {
        spa::emit_hook!($object.proxy().events(), $event $(, $($args),*)?)
    };
}

refcounted! {
    pub struct Proxy<T: HasProxy + Refcounted> {
        object: T::WeakRef,
        id: Id,
        bound_id: RefCell<Id>,
        hooks: Arc<Mutex<spa::hook::HookList<ProxyEvents>>>,
    }
}

#[derive(Default)]
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
            inner: Rc::new(InnerProxy::<T>::new(id, object.downgrade())),
        }
    }

    pub fn id(&self) -> Id {
        self.inner.id
    }

    pub fn object(&self) -> Option<T> {
        Refcounted::upgrade(&self.inner.object)
    }

    pub(crate) fn set_bound_id(&self, id: Id) {
        self.inner.bound_id.replace(id);
        spa::emit_hook!(self.inner.hooks, bound, id);
    }

    pub(crate) fn set_bound_props(&self, id: Id, props: &Properties) {
        self.inner.bound_id.replace(id);
        spa::emit_hook!(self.inner.hooks, bound_props, id, props);
    }

    pub fn add_listener(&self, events: ProxyEvents) {
        self.inner.hooks.lock().unwrap().append(events);
    }

    pub(crate) fn events(&self) -> Arc<Mutex<spa::hook::HookList<ProxyEvents>>> {
        self.inner.hooks.clone()
    }
}

impl<T: HasProxy + Refcounted> InnerProxy<T> {
    fn new(id: Id, object: T::WeakRef) -> Self {
        Self {
            object,
            id,
            bound_id: RefCell::new(INVALID_ID),
            hooks: spa::hook::HookList::new(),
        }
    }
}
