// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use crate::param::ParamObject;

use super::types::{Choice, Fd, Fraction, Id, ObjectType, Pointer, PropertyFlags, Rectangle, Type};
use super::{Error, Pod, Primitive, RawPod};

pub struct Parser<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(data: &'a [u8]) -> Parser<'a> {
        Parser { data, pos: 0 }
    }

    pub fn available(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn pop_pod<U: Pod>(&mut self) -> Result<<U as Pod>::DecodesTo, Error> {
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

    pub fn pop_string(&mut self) -> Result<String, Error> {
        self.pop_pod::<&str>()
    }

    pub fn pop_bytes(&mut self) -> Result<Vec<u8>, Error> {
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

    pub fn pop_array_raw<F>(&mut self, mut parse_item: F) -> Result<usize, Error>
    where
        F: FnMut(Type, &[u8]) -> Result<(), Error>,
    {
        if self.available() < 16 {
            return Err(Error::Invalid("Not enough data for array".to_string()));
        }

        let size =
            u32::from_ne_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap()) as usize;
        let total_size = 8 + size + super::pad_8(size);

        if self.available() < total_size {
            return Err(Error::Invalid("Not enough data for struct".to_string()));
        }

        let t = u32::from_ne_bytes(self.data[self.pos + 4..self.pos + 8].try_into().unwrap());
        if t != Type::Array as u32 {
            return Err(Error::Invalid(format!("Type {t} is not array")));
        }

        let child_size =
            u32::from_ne_bytes(self.data[self.pos + 8..self.pos + 12].try_into().unwrap()) as usize;
        let child_type =
            match u32::from_ne_bytes(self.data[self.pos + 12..self.pos + 16].try_into().unwrap())
                .try_into()
            {
                Ok(t) => t,
                Err(_) => return Err(Error::Invalid("Could noy parse child_type".to_string())),
            };

        let mut pos = self.pos + 16;

        while pos + child_size < total_size {
            parse_item(child_type, &self.data[pos..pos + child_size])?;
            pos += child_size;
        }

        self.pos += total_size;

        Ok(total_size)
    }

    pub fn pop_choice<T>(&mut self) -> Result<Choice<T>, Error>
    where
        T: Pod + Primitive,
    {
        self.pop_pod::<Choice<T>>()
    }

    pub fn pop_choice_raw<F>(&mut self, parse_choice: F) -> Result<usize, Error>
    where
        F: FnOnce(Type, Choice<&[u8]>) -> Result<(), Error>,
    {
        let data = &self.data[self.pos..];

        if data.len() < 24 {
            return Err(Error::Invalid("Not enough data for choice".to_string()));
        }

        let size = u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
        let padding = super::pad_8(size);

        if data.len() < 8 + size + padding {
            return Err(Error::Invalid("Not enough data for choice".to_string()));
        }

        if u32::from_ne_bytes(data[4..8].try_into().unwrap()) != Type::Choice as u32 {
            return Err(Error::Invalid(format!(
                "Type {} is not choice",
                u32::from_ne_bytes(data[4..8].try_into().unwrap())
            )));
        }

        let choice_type = u32::from_ne_bytes(data[8..12].try_into().unwrap());
        // flags is unused, so we don't decode it at [12..16]
        let child_size = u32::from_ne_bytes(data[16..20].try_into().unwrap()) as usize;
        let child_type = u32::from_ne_bytes(data[20..24].try_into().unwrap())
            .try_into()
            .map_err(|_| Error::Invalid("Invalid child type in choice".to_string()))?;

        let child_1 = 24..24 + child_size;
        let child_2 = 24 + child_size..24 + child_size * 2;
        let child_3 = 24 + child_size * 2..24 + child_size * 3;
        let child_4 = 24 + child_size * 3..24 + child_size * 4;

        let choice = match choice_type {
            0 => Choice::None(&data[child_1]),
            1 => {
                if size != 16 + child_size * 3 {
                    return Err(Error::Invalid(
                        "Not enough data for choice range".to_string(),
                    ));
                }

                let default = &data[child_1];
                let min = &data[child_2];
                let max = &data[child_3];

                Choice::Range { default, min, max }
            }
            2 => {
                if size != 16 + child_size * 4 {
                    return Err(Error::Invalid(
                        "Not enough data for choice step".to_string(),
                    ));
                }

                let default = &data[child_1];
                let min = &data[child_2];
                let max = &data[child_3];
                let step = &data[child_4];

                Choice::Step {
                    default,
                    min,
                    max,
                    step,
                }
            }
            3 => {
                let default = &data[child_1];
                let mut alternatives = Vec::new();

                for i in 1..(size - 16) / child_size {
                    alternatives.push(&data[24 + child_size * i..24 + child_size * (i + 1)]);
                }

                Choice::Enum {
                    default,
                    alternatives,
                }
            }
            4 => {
                if size != 16 + child_size * 2 {
                    return Err(Error::Invalid(
                        "Not enough data for choice flags".to_string(),
                    ));
                }

                let default = &data[child_1];
                let flags = &data[child_2];

                Choice::Flags { default, flags }
            }
            t => return Err(Error::Invalid(format!("Invalid choice type {t}"))),
        };

        parse_choice(child_type, choice)?;

        Ok(8 + size + padding)
    }

    pub fn pop_struct<F, T>(&mut self, parse_struct: F) -> Result<(T, usize), Error>
    where
        F: FnOnce(&mut Parser) -> Result<T, Error>,
    {
        if self.available() < 8 {
            return Err(Error::Invalid("Not enough data for struct".to_string()));
        }

        let size =
            u32::from_ne_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap()) as usize;
        if self.available() < 8 + size {
            return Err(Error::Invalid("Not enough data for struct".to_string()));
        }

        let t = u32::from_ne_bytes(self.data[self.pos + 4..self.pos + 8].try_into().unwrap());
        if t != Type::Struct as u32 {
            return Err(Error::Invalid(format!("Type {t} is not struct")));
        }

        let mut struct_parser = Parser::new(&self.data[self.pos + 8..self.pos + 8 + size]);
        let ret = parse_struct(&mut struct_parser)?;

        // The caller may or may not iterate over all fields, don't depend on that
        self.pos += size + 8;

        Ok((ret, size + 8))
    }

    pub fn pop_object<K, I, T>(
        &'a mut self,
        parse_object: impl FnOnce(&mut ObjectParser<'a, K>, I) -> Result<T, Error>,
    ) -> Result<(T, usize), Error>
    where
        K: ParamObject,
        I: TryFrom<u32>,
    {
        if self.available() < 16 {
            return Err(Error::Invalid("Not enough data for object".to_string()));
        }

        let size =
            u32::from_ne_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap()) as usize;
        if self.available() < 8 + size {
            return Err(Error::Invalid("Not enough data for object".to_string()));
        }

        let t = u32::from_ne_bytes(self.data[self.pos + 4..self.pos + 8].try_into().unwrap());
        if t != Type::Object as u32 {
            return Err(Error::Invalid(format!("Type {t} is not object")));
        }

        let object_type = match ObjectType::try_from(u32::from_ne_bytes(
            self.data[self.pos + 8..self.pos + 12].try_into().unwrap(),
        )) {
            Ok(ot) => ot,
            Err(e) => {
                return Err(Error::Invalid(format!(
                    "Could not decode object type: {e:?}"
                )))
            }
        };

        let id = match I::try_from(u32::from_ne_bytes(
            self.data[self.pos + 12..self.pos + 16].try_into().unwrap(),
        )) {
            Ok(id) => id,
            Err(_) => return Err(Error::Invalid("Could not decode id".to_string())),
        };

        if object_type != K::TYPE {
            return Err(Error::Invalid(format!(
                "Unexpected object type {object_type:?}"
            )));
        }

        self.pos += 16;

        let ret = {
            let mut object_parser = ObjectParser::new(&self.data[self.pos..self.pos + size - 8]);
            parse_object(&mut object_parser, id)?
        };

        // The caller may or may not iterate over all properties, don't depend on that
        self.pos += size - 8;

        Ok((ret, size + 8))
    }

    pub fn pop_object_raw<I, T>(
        &'a mut self,
        parse_object: impl FnOnce(&mut ObjectParserRaw<'a>, ObjectType, I) -> Result<T, Error>,
    ) -> Result<(T, usize), Error>
    where
        I: TryFrom<u32>,
    {
        if self.available() < 16 {
            return Err(Error::Invalid("Not enough data for object".to_string()));
        }

        let size =
            u32::from_ne_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap()) as usize;
        if self.available() < 8 + size {
            return Err(Error::Invalid("Not enough data for object".to_string()));
        }

        let t = u32::from_ne_bytes(self.data[self.pos + 4..self.pos + 8].try_into().unwrap());
        if t != Type::Object as u32 {
            return Err(Error::Invalid(format!("Type {t} is not object")));
        }

        let object_type = match ObjectType::try_from(u32::from_ne_bytes(
            self.data[self.pos + 8..self.pos + 12].try_into().unwrap(),
        )) {
            Ok(ot) => ot,
            Err(e) => {
                return Err(Error::Invalid(format!(
                    "Could not decode object type: {e:?}"
                )))
            }
        };

        let id = match I::try_from(u32::from_ne_bytes(
            self.data[self.pos + 12..self.pos + 16].try_into().unwrap(),
        )) {
            Ok(id) => id,
            Err(_) => return Err(Error::Invalid("Could not decode id".to_string())),
        };

        self.pos += 16;

        let ret = {
            let mut object_parser = ObjectParserRaw::new(&self.data[self.pos..self.pos + size - 8]);
            parse_object(&mut object_parser, object_type, id)?
        };

        // The caller may or may not iterate over all properties, don't depend on that
        self.pos += size - 8;

        Ok((ret, size + 8))
    }

    pub fn pop_raw_pod(&mut self) -> Result<RawPod<'a>, Error> {
        let res = RawPod::wrap(&self.data[self.pos..])?;

        self.pos += res.size;

        Ok(res)
    }
}

pub struct ObjectParser<'a, K> {
    data: &'a [u8],
    pos: usize,
    phantom: std::marker::PhantomData<K>,
}

impl<'a, K> ObjectParser<'a, K> {
    fn new(data: &'a [u8]) -> ObjectParser<'a, K> {
        ObjectParser {
            data,
            pos: 0,
            phantom: std::marker::PhantomData,
        }
    }

    pub fn available(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn pop_property(&mut self) -> Result<Option<(K, PropertyFlags, RawPod<'a>)>, Error>
    where
        K: TryFrom<u32> + ParamObject,
    {
        if self.available() == 0 {
            return Ok(None);
        }

        if self.available() < 16 {
            return Err(Error::Invalid(
                "Not enough data for object property".to_string(),
            ));
        }

        let key = match K::try_from(u32::from_ne_bytes(
            self.data[self.pos..self.pos + 4].try_into().unwrap(),
        )) {
            Ok(k) => k,
            Err(_) => return Err(Error::Invalid("Could not decode key".to_string())),
        };

        let flags = match PropertyFlags::from_bits(u32::from_ne_bytes(
            self.data[self.pos + 4..self.pos + 8].try_into().unwrap(),
        )) {
            Some(f) => f,
            None => return Err(Error::Invalid("Could not decode flags".to_string())),
        };

        self.pos += 8;

        let data = RawPod::wrap(&self.data[self.pos..])?;

        self.pos += data.total_size();

        Ok(Some((key, flags, data)))
    }
}

impl<'a, K: ParamObject + TryFrom<u32>> Iterator for ObjectParser<'a, K> {
    type Item = (K, PropertyFlags, RawPod<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.pop_property() {
            Ok(Some(item)) => Some(item),
            Ok(None) => None, // end of data
            Err(_) => None,   // actual parsing error
        }
    }
}

pub struct ObjectParserRaw<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> ObjectParserRaw<'a> {
    fn new(data: &'a [u8]) -> ObjectParserRaw<'a> {
        ObjectParserRaw { data, pos: 0 }
    }

    pub fn available(&self) -> usize {
        self.data.len() - self.pos
    }

    pub fn pop_property(&mut self) -> Result<Option<(u32, PropertyFlags, RawPod<'a>)>, Error> {
        if self.available() == 0 {
            return Ok(None);
        }

        if self.available() < 16 {
            return Err(Error::Invalid(
                "Not enough data for object property".to_string(),
            ));
        }

        let key = u32::from_ne_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());

        let flags = match PropertyFlags::from_bits(u32::from_ne_bytes(
            self.data[self.pos + 4..self.pos + 8].try_into().unwrap(),
        )) {
            Some(f) => f,
            None => return Err(Error::Invalid("Could not decode flags".to_string())),
        };

        self.pos += 8;

        let data = RawPod::wrap(&self.data[self.pos..])?;

        self.pos += data.total_size();

        Ok(Some((key, flags, data)))
    }
}

impl<'a> Iterator for ObjectParserRaw<'a> {
    type Item = (u32, PropertyFlags, RawPod<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        match self.pop_property() {
            Ok(Some(item)) => Some(item),
            Ok(None) => None, // end of data
            Err(_) => None,   // actual parsing error
        }
    }
}
