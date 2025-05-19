// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::ffi::{c_char, CStr, CString};

#[repr(C)]
struct Item {
    key: *mut c_char,
    value: *mut c_char,
}

#[repr(C)]
pub struct Dict {
    flags: u32,
    n_items: u32,
    items: *mut Item,
}

impl Dict {
    pub fn new(mut map: Vec<(String, String)>) -> Dict {
        if map.is_empty() {
            return Dict {
                flags: 0,
                n_items: 0,
                items: std::ptr::null_mut(),
            };
        }

        let items = map
            .drain(..)
            .map(|(k, v)| Item {
                key: CString::new(k).unwrap().into_raw(),
                value: CString::new(v).unwrap().into_raw(),
            })
            .collect::<Vec<Item>>()
            .into_boxed_slice();

        Dict {
            flags: 0,
            n_items: items.len() as u32,
            items: Box::into_raw(items) as *mut Item,
        }
    }

    pub fn items(&self) -> Vec<(&str, &str)> {
        let mut ret = vec![];
        let boxed = unsafe {
            let slice = std::slice::from_raw_parts_mut(self.items, self.n_items as usize);
            Box::from_raw(slice)
        };

        for i in &boxed {
            unsafe {
                ret.push((
                    CStr::from_ptr(i.key).to_str().unwrap(),
                    (CStr::from_ptr(i.value).to_str().unwrap()),
                ));
            }
        }

        // Unbox so we don't deallocate on drop
        let _ = Box::into_raw(boxed);

        ret
    }

    pub fn lookup(&self, key: &str) -> Option<&str> {
        let mut ret = None;
        let boxed = unsafe {
            let slice = std::slice::from_raw_parts_mut(self.items, self.n_items as usize);
            Box::from_raw(slice)
        };

        for i in &boxed {
            unsafe {
                if key.as_bytes() == CStr::from_ptr(i.key).to_bytes() {
                    ret = Some(CStr::from_ptr(i.value).to_str().unwrap());
                    break;
                }
            }
        }

        // Unbox so we don't deallocate on drop
        let _ = Box::into_raw(boxed);

        ret
    }

    pub fn as_raw(&self) -> *const Dict {
        self as *const Dict
    }
}

impl Drop for Dict {
    fn drop(&mut self) {
        if self.items.is_null() {
            return;
        }

        unsafe {
            let slice = std::slice::from_raw_parts_mut(self.items, self.n_items as usize);
            let boxed = Box::from_raw(slice);

            for ref mut item in boxed {
                let _ = CString::from_raw(item.key);
                let _ = CString::from_raw(item.value);
            }
        }
    }
}
