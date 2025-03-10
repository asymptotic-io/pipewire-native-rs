// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::ffi::c_void;
use std::os::fd::RawFd;

use super::error::Error;
use super::types::{
    Choice, Fd, Fraction, Id, ObjectType, Pointer, Property, PropertyFlags, Rectangle, Type,
};
use super::{Pod, Primitive};

pub struct Builder<'a> {
    data: &'a mut [u8],
    pos: usize,
    error: Option<Error>,
}

impl<'a> Builder<'a> {
    pub fn new(data: &'a mut [u8]) -> Self {
        Self {
            data,
            pos: 0,
            error: None,
        }
    }

    pub fn build(self) -> Result<&'a [u8], Error> {
        if let Some(e) = self.error {
            Err(e)
        } else {
            Ok(&self.data[0..self.pos])
        }
    }

    pub fn push_pod<U: Pod<'a>>(mut self, value: U) -> Self {
        if self.error.is_none() {
            match value.encode(&mut self.data[self.pos..]) {
                Ok(size) => self.pos += size,
                Err(e) => self.error = Some(e),
            }
        }

        self
    }

    pub fn push_none(self) -> Self {
        self.push_pod(())
    }

    pub fn push_bool(self, value: bool) -> Self {
        self.push_pod(value)
    }

    pub fn push_id<T>(self, value: Id<T>) -> Self
    where
        T: Into<u32> + TryFrom<u32> + Copy,
    {
        self.push_pod(value)
    }

    pub fn push_int(self, value: i32) -> Self {
        self.push_pod(value)
    }

    pub fn push_long(self, value: i64) -> Self {
        self.push_pod(value)
    }

    pub fn push_float(self, value: f32) -> Self {
        self.push_pod(value)
    }

    pub fn push_double(self, value: f64) -> Self {
        self.push_pod(value)
    }

    pub fn push_fd(self, value: RawFd) -> Self {
        self.push_pod(Fd(value))
    }

    pub fn push_rectangle(self, width: u32, height: u32) -> Self {
        self.push_pod(Rectangle { width, height })
    }

    pub fn push_fraction(self, num: u32, denom: u32) -> Self {
        self.push_pod(Fraction { num, denom })
    }

    pub fn push_string(self, value: &'a str) -> Self {
        self.push_pod(value)
    }

    pub fn push_bytes(self, value: &'a [u8]) -> Self {
        self.push_pod(value)
    }

    pub fn push_pointer(self, typ: Type, value: *const c_void) -> Self {
        self.push_pod(Pointer {
            type_: typ,
            ptr: value,
        })
    }

    pub fn push_array<T>(self, values: &[T]) -> Self
    where
        T: Pod<'a> + Primitive,
    {
        self.push_pod(values)
    }

    pub fn push_choice<T>(self, value: Choice<T>) -> Self
    where
        T: Pod<'a> + Primitive,
    {
        self.push_pod(value)
    }

    // Struct is encoded as
    //
    // +--------------+
    // |  total size  | 4 bytes
    // +--------------+
    // |   pod type   | 4 bytes
    // +--------------+
    // |              |
    // | member pods  | size bytes
    // |              |
    // +--------------+
    //
    pub fn push_struct<F>(mut self, build_struct: F) -> Self
    where
        F: FnOnce(StructBuilder) -> StructBuilder,
    {
        if self.error.is_some() {
            return self;
        }

        if self.data.len() < 8 {
            self.error = Some(Error::NoSpace);
            return self;
        }

        let old_pos = self.pos;
        self.pos += 8;

        let struct_builder = StructBuilder::new(self);
        let ret = build_struct(struct_builder).builder();

        if ret.error.is_some() {
            return ret;
        }

        let size = ret.pos - old_pos - 8;
        ret.data[old_pos..old_pos + 4].copy_from_slice(&(size as u32).to_ne_bytes());
        ret.data[old_pos + 4..old_pos + 8].copy_from_slice(&(Type::Struct as u32).to_ne_bytes());

        ret
    }

    // Object is encoded as
    //
    // +--------------+
    // |  total size  | 4 bytes
    // +--------------+
    // |   pod type   | 4 bytes
    // +--------------+
    // | object type  | 4 bytes
    // +--------------+
    // |  object id   | 4 bytes
    // +--------------+
    // |              |
    // |   member     |
    // |   props      |
    // |              |
    // +--------------+
    //
    // where each prop is
    //
    // +--------------+
    // |   key        | 4 bytes
    // +--------------+
    // |  flags       | 4 bytes
    // +--------------+
    // |              |
    // |   value      |
    // |   pod        |
    // |              |
    // +--------------+
    //
    pub fn push_object<T, F>(mut self, type_: ObjectType, id: T, build_object: F) -> Self
    where
        T: Into<u32> + TryFrom<u32>,
        F: FnOnce(ObjectBuilder) -> ObjectBuilder,
    {
        if self.error.is_some() {
            return self;
        }

        if self.data.len() < 16 {
            self.error = Some(Error::NoSpace);
            return self;
        }

        // Leave some space for the header
        let old_pos = self.pos;
        self.pos += 16;

        // Write out all the props
        let object_builder = build_object(ObjectBuilder::new(self));
        let ret = object_builder.builder();

        if ret.error.is_some() {
            return ret;
        }

        // Fill in the header
        let size = ret.pos - old_pos - 8;
        ret.data[old_pos..old_pos + 4].copy_from_slice(&(size as u32).to_ne_bytes());
        ret.data[old_pos + 4..old_pos + 8].copy_from_slice(&(Type::Object as u32).to_ne_bytes());
        ret.data[old_pos + 8..old_pos + 12].copy_from_slice(&(type_ as u32).to_ne_bytes());
        ret.data[old_pos + 12..old_pos + 16].copy_from_slice(&id.into().to_ne_bytes());

        ret
    }
}

pub struct StructBuilder<'a> {
    builder: Builder<'a>,
}

impl<'a> StructBuilder<'a> {
    fn new(builder: Builder<'a>) -> Self {
        Self { builder }
    }

    fn builder(self) -> Builder<'a> {
        self.builder
    }

    pub fn push_pod<U: Pod<'a>>(self, value: U) -> Self {
        StructBuilder::new(self.builder.push_pod(value))
    }

    pub fn push_none(self) -> Self {
        StructBuilder::new(self.builder.push_none())
    }

    pub fn push_bool(self, value: bool) -> Self {
        StructBuilder::new(self.builder.push_bool(value))
    }

    pub fn push_id<T>(self, value: Id<T>) -> Self
    where
        T: Into<u32> + TryFrom<u32> + Copy,
    {
        StructBuilder::new(self.builder.push_id(value))
    }

    pub fn push_int(self, value: i32) -> Self {
        StructBuilder::new(self.builder.push_int(value))
    }

    pub fn push_long(self, value: i64) -> Self {
        StructBuilder::new(self.builder.push_long(value))
    }

    pub fn push_float(self, value: f32) -> Self {
        StructBuilder::new(self.builder.push_float(value))
    }

    pub fn push_double(self, value: f64) -> Self {
        StructBuilder::new(self.builder.push_double(value))
    }

    pub fn push_fd(self, value: RawFd) -> Self {
        StructBuilder::new(self.builder.push_fd(value))
    }

    pub fn push_rectangle(self, width: u32, height: u32) -> Self {
        StructBuilder::new(self.builder.push_rectangle(width, height))
    }

    pub fn push_fraction(self, num: u32, denom: u32) -> Self {
        StructBuilder::new(self.builder.push_fraction(num, denom))
    }

    pub fn push_string(self, value: &'a str) -> Self {
        StructBuilder::new(self.builder.push_string(value))
    }

    pub fn push_bytes(self, value: &'a [u8]) -> Self {
        StructBuilder::new(self.builder.push_bytes(value))
    }

    pub fn push_pointer(self, typ: Type, value: *const c_void) -> Self {
        StructBuilder::new(self.builder.push_pointer(typ, value))
    }

    pub fn push_array<T>(self, values: &[T]) -> Self
    where
        T: Pod<'a> + Primitive,
    {
        StructBuilder::new(self.builder.push_array(values))
    }

    pub fn push_choice<T>(self, value: Choice<T>) -> Self
    where
        T: Pod<'a> + Primitive,
    {
        StructBuilder::new(self.builder.push_choice(value))
    }

    pub fn push_struct<F>(self, build_struct: F) -> Self
    where
        F: FnOnce(StructBuilder) -> StructBuilder,
    {
        StructBuilder::new(self.builder.push_struct(build_struct))
    }

    pub fn push_object<T, F>(self, type_: ObjectType, id: T, build_object: F) -> Self
    where
        T: Into<u32> + TryFrom<u32>,
        F: FnOnce(ObjectBuilder) -> ObjectBuilder,
    {
        StructBuilder::new(self.builder.push_object(type_, id, build_object))
    }
}

pub struct ObjectBuilder<'a> {
    builder: Builder<'a>,
}

impl<'a> ObjectBuilder<'a> {
    fn new(builder: Builder<'a>) -> Self {
        Self { builder }
    }

    fn builder(self) -> Builder<'a> {
        self.builder
    }

    pub fn push_property<K, V>(mut self, key: K, flags: PropertyFlags, value: V) -> Self
    where
        K: Copy + Into<u32> + TryFrom<u32> + 'a,
        V: Pod<'a>,
    {
        self.builder = self.builder.push_pod(Property { key, flags, value });
        self
    }
}
