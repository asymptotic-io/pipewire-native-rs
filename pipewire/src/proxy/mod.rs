// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::any::Any;
use std::sync::{Arc, Mutex, RwLock};

use pipewire_native_spa as spa;

use crate::{new_refcounted, properties::Properties, refcounted, Refcounted};

use crate::{types::ObjectType, Id};

/// A proxy representing client objects.
pub mod client;
/// A proxy representing device objects.
pub mod device;
/// A proxy representing modules loaded in the server.
pub mod module;
/// A proxy representing nodes.
pub mod node;
/// A proxy representing the registry.
pub mod registry;

refcounted! {
    /// Proxies are a central concept to how clients interact with a PipeWire server. The server
    /// has a list of objects (either instantiated locally, or by other clients). A number of these
    /// objects are exported to other clients to enumerate, interact with (call _methods_ on) and
    /// be notified when they change (via _events_).
    ///
    /// Clients discover exported objects using a [Registry](registry::Registry) (itself also a
    /// proxy), which can be created using [Core::registry()](super::core::Core::registry()). The
    /// [RegistryEvents::global](registry::RegistryEvents::global) event is triggered for each
    /// existing exported object (and subsequenty when new objects are created).
    ///
    /// If an object is of interest, clients can "bind" to that object using
    /// [Registry::bind()](registry::Registry::bind()). This provides an object on which methods
    /// may be called, and  event notifications may be received.
    ///
    /// The object itself will be a specific type (such as [Client](client::Client), but will also
    /// have an associated [Proxy\<T\>](Proxy) type (in this example, [Proxy\<Client\>](Proxy)).
    /// The specific type provides methods and events that are specific to the object itself. In
    /// addition, the [Proxy\<T\>](Proxy) type provides more general proxy-related methods and
    /// events.
    ///
    /// Specific types implement the [HasProxy] trait, which allows maintaining a generic
    /// collection of these objects, while having the ability to downcast to the specific type. The
    /// [HasProxy::proxy()] method can be used to get the corresponding [Proxy] object.
    ///
    /// Note that there are two IDs associated with a proxy. One is a "local" ID, which represents
    /// the client's view of the object. The other is a "global" ID, which is the server's view of
    /// the object. Due to the asynchronous nature of the protocol, it is possible that the client
    /// has a view of server objects that is not current. Global IDs might be reused, and this
    /// mechanism allows the server to keep track of what each client's view of the server state is
    /// and avoid calling methods on objects that have gone away.
    pub struct Proxy<T: HasProxy + Refcounted> {
        object: T::WeakRef,
        id: Id,
        bound_id: RwLock<Option<Id>>,
        hooks: Arc<Mutex<spa::hook::HookList<ProxyEvents>>>,
    }
}

/// Events that might be emitted by a proxy.
#[allow(clippy::type_complexity)]
#[derive(Default)]
pub struct ProxyEvents {
    /// The proxy is about to be destroyed (either because it went away, or because we are
    /// disconnecting).
    pub destroy: Option<Box<dyn FnMut()>>,
    /// The proxy was bound to.
    pub bound: Option<Box<dyn FnMut(Id)>>,
    /// The proxy was removed (either on the server-side, or we are disconnecting).
    pub removed: Option<Box<dyn FnMut()>>,
    /// An asynchronous operation on the proxy was completed.
    pub done: Option<Box<dyn FnMut(u32)>>,
    /// An error occured on the object.
    pub error: Option<Box<dyn FnMut(u32, u32, &str)>>,
    /// The proxy was bound to (supercedes [Self::bound]).
    pub bound_props: Option<Box<dyn FnMut(u32, &Properties)>>,
}

impl<T: HasProxy + Refcounted> Proxy<T> {
    pub(crate) fn new(id: Id, object: &T) -> Self {
        Self {
            inner: new_refcounted(InnerProxy::<T>::new(id, object.downgrade())),
        }
    }

    /// The "local" ID for this object.
    pub fn id(&self) -> Id {
        self.inner.id
    }

    /// Retrieves the specific object corresponding to this [Proxy]. Because the proxy holds a weak
    /// reference to the object, the returned value is an [Option].
    pub fn object(&self) -> Option<T> {
        Refcounted::upgrade(&self.inner.object)
    }

    /// The "global" ID for this object.
    pub fn bound_id(&self) -> Option<Id> {
        *self.inner.bound_id.read().unwrap()
    }

    pub(crate) fn set_bound_id(&self, id: Id) {
        *self.inner.bound_id.write().unwrap() = Some(id);
        spa::emit_hook!(self.inner.hooks, bound, id);
    }

    pub(crate) fn set_bound_props(&self, id: Id, props: &Properties) {
        *self.inner.bound_id.write().unwrap() = Some(id);
        spa::emit_hook!(self.inner.hooks, bound_props, id, props);
    }

    /// Register a listener for proxy events.
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
            bound_id: RwLock::new(None),
            hooks: spa::hook::HookList::new(),
        }
    }
}

/// This trait is implemented by all specific types of proxies. See the [Proxy] documentation for
/// more details.
pub trait HasProxy: Any {
    // See the invoke! and notify! macros below
    // type Methods;
    // type Events;

    /// The interface type of the proxy object.
    fn type_(&self) -> ObjectType;

    /// The interface version of the proxy object.
    fn version(&self) -> u32;

    /// Get a [Proxy\<T\>](Proxy) for this object.
    fn proxy(&self) -> Proxy<Self>
    where
        Self: Refcounted;
}

impl dyn HasProxy {
    /// Downcast from a `dyn HasProxy` to the specific type.
    pub fn downcast<T: HasProxy + Refcounted>(&self) -> Option<T> {
        (self as &dyn Any).downcast_ref::<T>().cloned()
    }

    /// Downcast from a `dyn HasProxy` to the corresponding [Proxy] type.
    pub fn downcast_proxy<T: HasProxy + Refcounted>(&self) -> Option<Proxy<T>> {
        (self as &dyn Any).downcast_ref::<T>().map(|o| o.proxy())
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

#[doc(hidden)]
#[macro_export]
macro_rules! proxy_object_invoke {
    ($proxy:ident, $method:ident $(, $($args:tt)*)?) => {
        ($proxy.object().unwrap().methods().lock().unwrap().$method)(&$proxy $(, $($args)*)?)
    };
}

#[doc(hidden)]
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
#[doc(hidden)]
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
            } else if $object.type_() == $crate::types::interface::DEVICE {
                let _proxy = $object.downcast_proxy::<$crate::proxy::device::Device>().unwrap();
                _proxy.$method($($($args),*)?)
            } else if $object.type_() == $crate::types::interface::MODULE {
                let _proxy = $object.downcast_proxy::<$crate::proxy::module::Module>().unwrap();
                _proxy.$method($($($args),*)?)
            } else if $object.type_() == $crate::types::interface::NODE {
                let _proxy = $object.downcast_proxy::<$crate::proxy::node::Node>().unwrap();
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

#[doc(hidden)]
#[macro_export]
macro_rules! hasproxy_notify {
    ($object:ident, $event:ident $(, $($args:tt),*)?) => {
        if $object.type_() == $crate::types::interface::CORE {
            let _proxy = $object.downcast_proxy::<$crate::core::Core>().unwrap();
            spa::emit_hook!(_proxy.events(), $event $(, $($args),*)?)
        } else if $object.type_() == $crate::types::interface::CLIENT {
            let _proxy = $object.downcast_proxy::<$crate::proxy::client::Client>().unwrap();
            spa::emit_hook!(_proxy.events(), $event $(, $($args),*)?)
        } else if $object.type_() == $crate::types::interface::DEVICE {
            let _proxy = $object.downcast_proxy::<$crate::proxy::device::Device>().unwrap();
            spa::emit_hook!(_proxy.events(), $event $(, $($args),*)?)
        } else if $object.type_() == $crate::types::interface::MODULE {
            let _proxy = $object.downcast_proxy::<$crate::proxy::module::Module>().unwrap();
            spa::emit_hook!(_proxy.events(), $event $(, $($args),*)?)
        } else if $object.type_() == $crate::types::interface::NODE {
            let _proxy = $object.downcast_proxy::<$crate::proxy::node::Node>().unwrap();
            spa::emit_hook!(_proxy.events(), $event $(, $($args),*)?)
        } else if $object.type_() == $crate::types::interface::REGISTRY {
            let _proxy = $object.downcast_proxy::<$crate::proxy::registry::Registry>().unwrap();
            spa::emit_hook!(_proxy.events(), $event $(, $($args),*)?)
        } else {
            unreachable!("got unexpected proxy type {}", $object.type_())
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! proxy_notify {
    ($object:ident, $event:ident $(, $($args:tt),*)?) => {
        spa::emit_hook!($object.proxy().events(), $event $(, $($args),*)?)
    };
}
