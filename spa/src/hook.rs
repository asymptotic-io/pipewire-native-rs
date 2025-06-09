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
    // The return value is an Rc<RefCell<...>> because:
    //
    //   1. Rc<> allows us to clone() the hooklist before emission, so that we can mutably borrow
    //      the list (because the callbacks structure needs to be mutably borrowed, as the callback
    //      can be an FnMut, which needs to be mutably borrowed when called)
    //
    //   2. RefCell<> is then needed inside the Rc, so that we can mutate it at all.
    //
    // We might want to explore alternatives that let us push the RefCell<> all the way into the
    // callbacks structure itself, so each callback can individually be mutably borrowed, so that
    // one callback can call another callback if needed.
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
    ($hook_list:expr, $method:ident, $($args:tt)*) => {
        {
            let _h = $hook_list.clone();
            let mut _h = _h.lock();
            let _hooks = _h.as_deref_mut().unwrap();

            for h in _hooks.iter_mut() {
                (h.callbacks().$method)($($args)*);
            }
        }
    };
}
