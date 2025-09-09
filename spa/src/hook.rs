// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    collections::LinkedList,
    sync::{Arc, Mutex},
};

pub type HookId = u32;

pub struct Hook<T> {
    id: HookId,
    callbacks: T,
}

impl<T> Hook<T> {
    pub fn callbacks(&mut self) -> &mut T {
        &mut self.callbacks
    }
}

pub struct HookList<T> {
    hooks: LinkedList<Hook<T>>,
    next_id: HookId,
}

impl<T> HookList<T> {
    // The return value is an Arc<Mutex<...>> for a few reasons.
    //
    // Firstly, the hook list is usually stored in the structure on which the hook is being emitted
    // -- that is, the structure is usually being captued for the closure that is going into the
    // hooks. This means that the closure might need to have a mutable borrow of the object itself,
    // like this example from tests/hook.rs
    //
    // ```
    //   // The hook list is contained in `this`, and the hook takes an &mut this as the first
    //   // argument
    //   emit_hook!(this.hooks, mutie, &mut this, ...);
    // ```
    //
    // If the hook list was not Clone, then we would need to take a reference to `this` above, and
    // that would not let us also take a mutable reference for the closure argument.
    //
    // Now given we need to be able to clone the hook list, the inner structures need to be mutable
    // (so we can add and remove hooks), which means a Mutex is necessary.
    //
    // A second reason for that mutability is that closure functions might be FnMut, which means
    // we need to mutably borrow the callbacks list in order to call the function at all.
    pub fn new() -> Arc<Mutex<HookList<T>>> {
        Arc::new(Mutex::new(HookList {
            hooks: LinkedList::new(),
            next_id: 0,
        }))
    }

    pub fn prepend(&mut self, callbacks: T) -> HookId {
        let id = self.next_id;
        let hook = Hook { id, callbacks };

        self.hooks.push_front(hook);
        self.next_id += 1;

        id
    }

    pub fn append(&mut self, callbacks: T) -> HookId {
        let id = self.next_id;
        let hook = Hook { id, callbacks };

        self.hooks.push_back(hook);
        self.next_id += 1;

        id
    }

    // We only implement `iter_mut()` because we expect T to contain `FnMut`s, which need to be
    // borrowed mutably while being called (as they might mutate captured variables in their
    // context)
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Hook<T>> {
        self.hooks.iter_mut()
    }

    pub fn remove(&mut self, id: HookId) -> Option<T> {
        self.hooks
            .extract_if(|h| h.id == id)
            .next()
            .map(|h| h.callbacks)
    }
}

#[macro_export]
macro_rules! emit_hook {
    ($hook_list:expr, $method:ident) => {
        {
            let _h = $hook_list.clone();
            let mut _h = _h.lock();
            let _hooks = _h.as_deref_mut().unwrap();

            for h in _hooks.iter_mut() {
                if let Some(_method) = h.callbacks().$method.as_deref_mut() {
                    (_method)();
                }
            }
        }
    };
    ($hook_list:expr, $method:ident, $($args:tt)*) => {
        {
            let _h = $hook_list.clone();
            let mut _h = _h.lock();
            let _hooks = _h.as_deref_mut().unwrap();

            for h in _hooks.iter_mut() {
                if let Some(_method) = h.callbacks().$method.as_deref_mut() {
                    (_method)($($args)*);
                }
            }
        }
    };
}
