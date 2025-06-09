// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    rc::Rc,
    sync::{Arc, Mutex},
};

use pipewire_native_spa::{emit_hook, hook::HookList};

struct TestEvents {
    constie: Box<dyn FnMut(i32)>,
    normie: Box<dyn FnMut(&TestStruct, i32, &str)>,
    mutie: Box<dyn FnMut(&mut TestStruct, i32, &str)>,
}

struct TestStruct {
    value: i32,
    name: String,
    hooks: Arc<Mutex<HookList<TestEvents>>>,
}

#[test]
fn test_hooks() {
    let accum = Rc::new(Mutex::new(0i32));

    let accum1 = accum.clone();
    let accum2 = accum.clone();

    let events = TestEvents {
        // Increment accumulator by callback value
        constie: Box::new(move |i| *accum1.lock().unwrap() += i),
        // Increment accumulator by struct value * callback value
        normie: Box::new(move |this, i, s| {
            *accum2.lock().unwrap() += this.value * i;
            assert_eq!(s, &this.name);
        }),
        // Set struct value to callback value
        mutie: Box::new(move |this, i, s| {
            this.value = i;
            this.name = s.to_string();
        }),
    };

    let mut this = TestStruct {
        value: 1,
        name: "First".to_string(),
        hooks: HookList::new(),
    };

    let id = this.hooks.lock().unwrap().append(events);

    emit_hook!(this.hooks, constie, 1);
    assert_eq!(*accum.lock().unwrap(), 1);

    emit_hook!(this.hooks, normie, &this, 2, "First");
    assert_eq!(*accum.lock().unwrap(), 3);

    emit_hook!(this.hooks, mutie, &mut this, 4, "Second");
    assert_eq!(this.value, 4);
    assert_eq!(this.name, "Second");

    this.hooks.lock().unwrap().remove(id);

    emit_hook!(this.hooks, constie, 1);
    assert_eq!(*accum.lock().unwrap(), 3);
}
