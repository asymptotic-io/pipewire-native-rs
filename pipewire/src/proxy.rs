// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::rc::{Rc, Weak};

use crate::{refcounted, Refcounted};

refcounted! {
    pub struct Proxy<T: Refcounted> {
        object: <T as Refcounted>::WeakRef,
    }
}

#[allow(private_bounds)]
impl<T: Refcounted> Proxy<T> {
    pub(crate) fn new(object: &T) -> Self {
        Self {
            inner: Rc::new(InnerProxy::<T>::new(object)),
        }
    }

    pub fn object(&self) -> Option<T> {
        <T as Refcounted>::upgrade(&self.inner.object)
    }
}

impl<T: Refcounted> InnerProxy<T> {
    fn new(object: &T) -> Self {
        Self {
            object: object.downgrade(),
        }
    }
}
