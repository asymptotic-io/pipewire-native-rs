// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use super::error::Error;
use super::types::{
    Choice, Fd, Fraction, Id, ObjectType, Pointer, Property, PropertyFlags, Rectangle, Type,
};
use super::{Pod, Primitive};

pub struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(data: &'a [u8]) -> Parser<'a> {
        Parser { data, pos: 0 }
    }

    pub fn pop_pod<U: Pod>(&mut self) -> Result<<U as Pod>::DecodesTo<'_>, Error> {
        let (res, size) = U::decode(&self.data[self.pos..])?;

        self.pos += size;

        Ok(res)
    }

    pub fn pop_none(&mut self) -> Result<(), Error> {
        self.pop_pod::<()>()
    }

    pub fn pop_bool(&mut self) -> Result<bool, Error> {
        self.pop_pod::<bool>()
    }

    pub fn pop_id<T>(&mut self) -> Result<Id<T>, Error>
    where
        T: Into<u32> + TryFrom<u32> + Copy,
    {
        self.pop_pod::<Id<T>>()
    }

    pub fn pop_int(&mut self) -> Result<i32, Error> {
        self.pop_pod::<i32>()
    }

    pub fn pop_long(&mut self) -> Result<i64, Error> {
        self.pop_pod::<i64>()
    }

    pub fn pop_float(&mut self) -> Result<f32, Error> {
        self.pop_pod::<f32>()
    }

    pub fn pop_double(&mut self) -> Result<f64, Error> {
        self.pop_pod::<f64>()
    }

    pub fn pop_string(&mut self) -> Result<&str, Error> {
        self.pop_pod::<&str>()
    }

    pub fn pop_bytes(&mut self) -> Result<&[u8], Error> {
        self.pop_pod::<&[u8]>()
    }

    pub fn pop_pointer(&mut self) -> Result<Pointer, Error> {
        self.pop_pod::<Pointer>()
    }

    pub fn pop_fd(&mut self) -> Result<Fd, Error> {
        self.pop_pod::<Fd>()
    }

    pub fn pop_rectangle(&mut self) -> Result<Rectangle, Error> {
        self.pop_pod::<Rectangle>()
    }

    pub fn pop_fraction(&mut self) -> Result<Fraction, Error> {
        self.pop_pod::<Fraction>()
    }

    pub fn pop_array<T>(&mut self) -> Result<Vec<T>, Error>
    where
        T: Pod + Primitive,
    {
        self.pop_pod::<&[T]>()
    }

    pub fn pop_choice<T>(&mut self) -> Result<Choice<T>, Error>
    where
        T: Pod + Primitive,
    {
        self.pop_pod::<Choice<T>>()
    }

    pub fn pop_struct<F>(&mut self, parse_struct: F) -> Result<(), Error>
    where
        F: FnOnce(&mut Parser) -> Result<(), Error>,
    {
        if self.data.len() < 8 {
            return Err(Error::Invalid);
        }

        let size =
            u32::from_ne_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap()) as usize;
        if self.data.len() < 8 + size {
            return Err(Error::Invalid);
        }

        let t = u32::from_ne_bytes(self.data[self.pos + 4..self.pos + 8].try_into().unwrap());
        if t != Type::Struct as u32 {
            return Err(Error::Invalid);
        }

        let mut struct_parser = Parser::new(&self.data[self.pos + 8..self.pos + 8 + size]);
        parse_struct(&mut struct_parser)?;

        self.pos += struct_parser.pos;

        Ok(())
    }

    pub fn pop_object<F, T>(&'a mut self, parse_object: F) -> Result<(), Error>
    where
        F: FnOnce(&mut ObjectParser<'a>, ObjectType, T) -> Result<(), Error>,
        T: Into<u32> + TryFrom<u32>,
    {
        if self.data.len() < 16 {
            return Err(Error::Invalid);
        }

        let size =
            u32::from_ne_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap()) as usize;
        if self.data.len() < 8 + size {
            return Err(Error::Invalid);
        }

        let t = u32::from_ne_bytes(self.data[self.pos + 4..self.pos + 8].try_into().unwrap());
        if t != Type::Object as u32 {
            return Err(Error::Invalid);
        }

        let object_type = match ObjectType::try_from(u32::from_ne_bytes(
            self.data[self.pos + 8..self.pos + 12].try_into().unwrap(),
        )) {
            Ok(ot) => ot,
            Err(_) => return Err(Error::Invalid),
        };

        let id = match T::try_from(u32::from_ne_bytes(
            self.data[self.pos + 12..self.pos + 16].try_into().unwrap(),
        )) {
            Ok(id) => id,
            Err(_) => return Err(Error::Invalid),
        };

        self.pos += 16;
        let mut object_parser = ObjectParser::new(self);

        parse_object(&mut object_parser, object_type, id)?;

        Ok(())
    }
}

pub struct ObjectParser<'a> {
    parser: &'a mut Parser<'a>,
}

impl<'a> ObjectParser<'a> {
    fn new(parser: &'a mut Parser<'a>) -> ObjectParser<'a> {
        ObjectParser { parser }
    }

    pub fn pop_property<K, V>(&mut self) -> Result<Property<K, <V as Pod>::DecodesTo<'_>>, Error>
    where
        K: Copy + Into<u32> + TryFrom<u32>,
        V: Pod,
    {
        if self.parser.data.len() < 8 {
            return Err(Error::Invalid);
        }

        let key = match K::try_from(u32::from_ne_bytes(
            self.parser.data[self.parser.pos..self.parser.pos + 4]
                .try_into()
                .unwrap(),
        )) {
            Ok(k) => k,
            Err(_) => return Err(Error::Invalid),
        };

        let flags = match PropertyFlags::from_bits(u32::from_ne_bytes(
            self.parser.data[self.parser.pos + 4..self.parser.pos + 8]
                .try_into()
                .unwrap(),
        )) {
            Some(f) => f,
            None => return Err(Error::Invalid),
        };

        self.parser.pos += 8;

        let value = self.parser.pop_pod::<V>()?;

        Ok(Property { key, flags, value })
    }
}
