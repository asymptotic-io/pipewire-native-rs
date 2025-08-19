// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

/// A helper trait for refcounted objects. Reccounted items can more easily be lifted into closures
/// without leaking data. This is accomplished by distinguising between "strong references" (cause
/// the object to exist) and "weak referenes" (which do not prevent the object from being freed).
///
/// There is a [`refcounted!`] macro to make declaring such types easier. A new strong reference
/// can cheaply be created using the `[Clone]` trait.
///
/// Example usage:
///
/// ```ignore
/// let registry = core.registry();
/// // Generate a weak reference, so the closure does not force the object to be alive forever.
/// let registry_weak = registry.downgrade();
///
/// registry.add_listener(ProxyEvents {
///     // Use the some_closure!() macro to avoid this boilerplate
///     global: move |id, _perms, type_, version, props| {
///         let registry = registry_weak
///             .upgrade()
///             .expect("registry hook should not outlive registry");
///         let object = registry.bind(id, type_, version);
///         // ...
///     },
///     ..Default::default()
/// });
/// ```
///
/// The `[closure!]` macro can be used to reduce the boilerplate of going through the
/// `downgrade()`/`upgrade()` cycle, especially for values that need to be sent into multiple
/// closures.
pub trait Refcounted: Clone {
    /// The type of a weak reference to the object
    type WeakRef;

    /// Try to convert a weak reference to a strong reference. If the underlying object isstill
    /// alive, returns a `Some` continaing the value. If the underlying object's strong reference
    /// count dropped to zero, and was thus freed, this returns `None`.
    fn upgrade(this: &Self::WeakRef) -> Option<Self>
    where
        Self: Sized;

    /// Create a weak reference to the object. This reference does not impact the object's
    /// lifecycle, and merely allows us the option to try to retrieve the object using
    /// [`Self::upgrade()`].
    fn downgrade(&self) -> Self::WeakRef;
}

pub(crate) fn new_refcounted<T>(inner: T) -> std::sync::Arc<T> {
    std::sync::Arc::new(inner)
}

#[macro_export]
/// This macro simplifies the creation of structs using the refcounted inner pattern. The rough
/// structure of such a struct is:
///
/// ```
/// struct Thing {
///     inner: std::sync::Arc<InnerThing>,
/// }
///
/// struct InnerThing {
///     id: u32,
///     name: String,
/// }
/// ```
///
/// The macro creates structures in this pattern, and implements the [`super::Refcounted`] trait on
/// the generated types.
macro_rules! refcounted {
    (
        // FIXME: bounds can be non-types, so we probably need something that munches tts
        $(#[$($attrs:meta)*])?
        $visibility:vis struct $name:ident $(<$($generic:ident $(: $bound:ty)?),*>)? {
            $($body:tt)*
        }
    ) => {
        paste::paste! {
            $(#[$($attrs)*])?
            #[derive(Clone)]
            $visibility struct $name $(<$($generic $(: $bound)?),*>)? {
                inner: std::sync::Arc<[<Inner $name>] $(<$($generic),*>)?>,
            }

            // We implement Send to allow usage with our main loop, and expect the user to
            // explicitly ensure single-thread access
            unsafe impl $(<$($generic $(: $bound)?),*>)? Send for $name $(<$($generic),*>)? {}

            #[derive(Clone)]
            /// A weak reference to $name
            pub struct [<Weak $name>] $(<$($generic $(: $bound)?),*>)? {
                inner: std::sync::Weak<[<Inner $name>] $(<$($generic>)?),*>,
            }

            // We implement Send to allow usage with our main loop, and expect the user to
            // explicitly ensure single-thread access
            unsafe impl $(<$($generic $(: $bound)?),*>)? Send for [<Weak $name>] $(<$($generic),*>)? {}

            impl $(<$($generic $(: $bound)?),*>)? $name $(<$($generic),*>)? {
                /// Helper method to generate a weak reference. See also: [`Refcounted`].
                pub fn downgrade(&self) -> [<Weak $name>] $(<$($generic),*>)? {
                    [<Weak $name>] {
                        inner: std::sync::Arc::downgrade(&self.inner),
                    }
                }
            }

            impl $(<$($generic $(: $bound)?),*>)? [<Weak $name>] $(<$($generic),*>)? {
                /// Helper method to convert a weak reference to a strong reference. See also:
                /// [`Refcounted`].
                pub fn upgrade(&self) -> Option<$name $(<$($generic),*>)?> {
                    self.inner.upgrade().map(|inner| $name { inner })
                }
            }

            impl $(<$($generic $(: $bound)?),*>)? crate::Refcounted for $name $(<$($generic),*>)? {
                type WeakRef = [<Weak $name>] $(<$($generic),*>)?;

                fn upgrade(this: &Self::WeakRef) -> Option<Self> {
                    this.upgrade()
                }

                fn downgrade(&self) -> Self::WeakRef {
                    self.downgrade()
                }
            }

            struct [<Inner $name>] $(<$($generic $(: $bound)?),*>)? {
                $($body)*
            }
        }
    }
}
