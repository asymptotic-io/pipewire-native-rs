// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

/// A helper trait for refcounted objects.
pub trait Refcounted: Clone {
    type WeakRef;

    fn upgrade(this: &Self::WeakRef) -> Option<Self>
    where
        Self: Sized;
    fn downgrade(&self) -> Self::WeakRef;
}

pub(crate) fn new_refcounted<T>(inner: T) -> std::sync::Arc<T> {
    std::sync::Arc::new(inner)
}

#[macro_export]
macro_rules! refcounted {
    (
        // FIXME: bounds can be non-types, so we probably need something that munches tts
        $(#[$(attrs:meta)+])?
        $visibility:vis struct $name:ident $(<$($generic:ident $(: $bound:ty)?),*>)? {
            $($body:tt)*
        }
    ) => {
        paste::paste! {
            #[derive(Clone)]
            $visibility struct $name $(<$($generic $(: $bound)?),*>)? {
                inner: std::sync::Arc<[<Inner $name>] $(<$($generic),*>)?>,
            }

            // We implement Send to allow usage with our main loop, and expect the user to
            // explicitly ensure single-thread access
            unsafe impl $(<$($generic $(: $bound)?),*>)? Send for $name $(<$($generic),*>)? {}

            #[derive(Clone)]
            pub struct [<Weak $name>] $(<$($generic $(: $bound)?),*>)? {
                inner: std::sync::Weak<[<Inner $name>] $(<$($generic>)?),*>,
            }

            // We implement Send to allow usage with our main loop, and expect the user to
            // explicitly ensure single-thread access
            unsafe impl $(<$($generic $(: $bound)?),*>)? Send for [<Weak $name>] $(<$($generic),*>)? {}

            impl $(<$($generic $(: $bound)?),*>)? $name $(<$($generic),*>)? {
                pub fn downgrade(&self) -> [<Weak $name>] $(<$($generic),*>)? {
                    [<Weak $name>] {
                        inner: std::sync::Arc::downgrade(&self.inner),
                    }
                }
            }

            impl $(<$($generic $(: $bound)?),*>)? [<Weak $name>] $(<$($generic),*>)? {
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
