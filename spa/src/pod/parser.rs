// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use super::error::Error;
use super::types::{
    Choice, Fd, Fraction, Id, ObjectType, Pointer, Property, PropertyFlags, Rectangle, Type,
};
use super::{Pod, Primitive};
use std::cell::RefCell;

pub struct Parser<'a> {
    data: &'a [u8],
    pos: RefCell<usize>,
}

impl<'a> Parser<'a> {
    pub fn new(data: &'a [u8]) -> Parser<'a> {
        Parser {
            data,
            pos: RefCell::new(0),
        }
    }

    pub fn pop_pod<U: Pod>(&self) -> Result<<U as Pod>::DecodesTo, Error> {
        let pos = *self.pos.borrow();

        let (res, size) = U::decode(&self.data[pos..])?;

        self.pos.replace_with(|&mut old| old + size);

        Ok(res)
    }

    pub fn pop_none(&self) -> Result<(), Error> {
        self.pop_pod::<()>()
    }

    pub fn pop_bool(&self) -> Result<bool, Error> {
        self.pop_pod::<bool>()
    }

    pub fn pop_id<T>(&self) -> Result<Id<T>, Error>
    where
        T: Into<u32> + TryFrom<u32> + Copy,
    {
        self.pop_pod::<Id<T>>()
    }

    pub fn pop_int(&self) -> Result<i32, Error> {
        self.pop_pod::<i32>()
    }

    pub fn pop_long(&self) -> Result<i64, Error> {
        self.pop_pod::<i64>()
    }

    pub fn pop_float(&self) -> Result<f32, Error> {
        self.pop_pod::<f32>()
    }

    pub fn pop_double(&self) -> Result<f64, Error> {
        self.pop_pod::<f64>()
    }

    pub fn pop_string(&self) -> Result<String, Error> {
        self.pop_pod::<&str>()
    }

    pub fn pop_bytes(&self) -> Result<Vec<u8>, Error> {
        self.pop_pod::<&[u8]>()
    }

    pub fn pop_pointer(&self) -> Result<Pointer, Error> {
        self.pop_pod::<Pointer>()
    }

    pub fn pop_fd(&self) -> Result<Fd, Error> {
        self.pop_pod::<Fd>()
    }

    pub fn pop_rectangle(&self) -> Result<Rectangle, Error> {
        self.pop_pod::<Rectangle>()
    }

    pub fn pop_fraction(&self) -> Result<Fraction, Error> {
        self.pop_pod::<Fraction>()
    }

    pub fn pop_array<T>(&self) -> Result<Vec<T>, Error>
    where
        T: Pod + Primitive,
    {
        self.pop_pod::<&[T]>()
    }

    pub fn pop_choice<T>(&self) -> Result<Choice<T>, Error>
    where
        T: Pod + Primitive,
    {
        self.pop_pod::<Choice<T>>()
    }

    pub fn pop_struct<F>(&self, parse_struct: F) -> Result<(), Error>
    where
        F: FnOnce(&mut Parser) -> Result<(), Error>,
    {
        if self.data.len() < 8 {
            return Err(Error::Invalid);
        }

        let pos = *self.pos.borrow();

        let size = u32::from_ne_bytes(self.data[pos..pos + 4].try_into().unwrap()) as usize;
        if self.data.len() < 8 + size {
            return Err(Error::Invalid);
        }

        let t = u32::from_ne_bytes(self.data[pos + 4..pos + 8].try_into().unwrap());
        if t != Type::Struct as u32 {
            return Err(Error::Invalid);
        }

        let mut struct_parser = Parser::new(&self.data[pos + 8..pos + 8 + size]);
        parse_struct(&mut struct_parser)?;

        let sp_pos = *(struct_parser.pos.borrow());
        self.pos.replace_with(|&mut old| old + sp_pos);

        Ok(())
    }

    pub fn pop_object<F, T>(&'a self, parse_object: F) -> Result<(), Error>
    where
        F: FnOnce(&mut ObjectParser<'a>, ObjectType, T) -> Result<(), Error>,
        T: Into<u32> + TryFrom<u32>,
    {
        if self.data.len() < 16 {
            return Err(Error::Invalid);
        }

        let pos = *self.pos.borrow();
        let size = u32::from_ne_bytes(self.data[pos..pos + 4].try_into().unwrap()) as usize;
        if self.data.len() < 8 + size {
            return Err(Error::Invalid);
        }

        let t = u32::from_ne_bytes(self.data[pos + 4..pos + 8].try_into().unwrap());
        if t != Type::Object as u32 {
            return Err(Error::Invalid);
        }

        let object_type = match ObjectType::try_from(u32::from_ne_bytes(
            self.data[pos + 8..pos + 12].try_into().unwrap(),
        )) {
            Ok(ot) => ot,
            Err(_) => return Err(Error::Invalid),
        };

        let id = match T::try_from(u32::from_ne_bytes(
            self.data[pos + 12..pos + 16].try_into().unwrap(),
        )) {
            Ok(id) => id,
            Err(_) => return Err(Error::Invalid),
        };

        self.pos.replace_with(|&mut old| old + 16);

        let mut object_parser = ObjectParser::new(self);

        parse_object(&mut object_parser, object_type, id)?;

        Ok(())
    }
}

pub struct ObjectParser<'a> {
    parser: &'a Parser<'a>,
}

impl<'a> ObjectParser<'a> {
    fn new(parser: &'a Parser<'a>) -> ObjectParser<'a> {
        ObjectParser { parser }
    }

    pub fn pop_property<K, V>(&self) -> Result<Property<K, <V as Pod>::DecodesTo>, Error>
    where
        K: Copy + Into<u32> + TryFrom<u32>,
        V: Pod,
    {
        if self.parser.data.len() < 8 {
            return Err(Error::Invalid);
        }

        let pos = *self.parser.pos.borrow();

        let key = match K::try_from(u32::from_ne_bytes(
            self.parser.data[pos..pos + 4].try_into().unwrap(),
        )) {
            Ok(k) => k,
            Err(_) => return Err(Error::Invalid),
        };

        let flags = match PropertyFlags::from_bits(u32::from_ne_bytes(
            self.parser.data[pos + 4..pos + 8].try_into().unwrap(),
        )) {
            Some(f) => f,
            None => return Err(Error::Invalid),
        };

        self.parser.pos.replace_with(|&mut old| old + 8);

        let value = self.parser.pop_pod::<V>()?;

        Ok(Property { key, flags, value })
    }
}
