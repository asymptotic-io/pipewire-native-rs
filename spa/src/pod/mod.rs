// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

pub mod builder;
pub mod parser;
pub mod types;

use std::ffi::c_void;
use std::os::fd::RawFd;

use types::{Choice, Fd, Fraction, Id, Pointer, Property, PropertyFlags, Rectangle, Type};

#[derive(Debug)]
pub enum Error {
    Invalid,
    NoSpace,
}

pub trait Pod {
    // Default to Self once that is stable, or try to generate references to owned data
    type DecodesTo;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error>;
    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), Error>;
}

pub trait Primitive {
    fn pod_type() -> Type;
    fn pod_size() -> usize;

    fn encode_body(&self, data: &mut [u8]) -> Result<(), Error>;
    fn decode_body(data: &[u8]) -> Result<Self, Error>
    where
        Self: Sized;
}

fn pad_8(size: usize) -> usize {
    if size % 8 > 0 {
        8 - size % 8
    } else {
        0
    }
}

pub struct RawPod<'a> {
    size: usize,
    type_: Type,
    data: &'a [u8],
}

impl<'a> RawPod<'a> {
    pub fn wrap(data: &'a [u8]) -> Result<RawPod<'a>, Error> {
        if data.len() < 8 {
            return Err(Error::NoSpace);
        }

        let internal_size = u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
        let size = 8 + internal_size + pad_8(internal_size);

        if size > data.len() {
            return Err(Error::NoSpace);
        }

        let type_ = Type::try_from(u32::from_ne_bytes(data[4..8].try_into().unwrap()))
            .map_err(|_| Error::Invalid)?;

        Ok(RawPod {
            size,
            type_,
            data: &data[0..size],
        })
    }

    pub fn total_size(&self) -> usize {
        self.size
    }

    pub fn type_(&self) -> Type {
        self.type_
    }

    pub fn decode<T>(&self) -> Result<<T as Pod>::DecodesTo, Error>
    where
        T: Pod,
    {
        T::decode(self.data).map(|v| v.0)
    }
}

impl<T> Pod for T
where
    T: Primitive,
{
    type DecodesTo = Self;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error> {
        let size = Self::pod_size();
        let padding = pad_8(size);

        if data.len() < 8 + size + padding {
            return Err(Error::NoSpace);
        }

        data[0..4].copy_from_slice(&(Self::pod_size() as u32).to_ne_bytes());
        data[4..8].copy_from_slice(&(Self::pod_type() as u32).to_ne_bytes());

        self.encode_body(&mut data[8..])?;

        if padding > 0 {
            data[8 + size..8 + size + padding].copy_from_slice(&[0; 8][0..padding]);
        }

        Ok(8 + size + padding)
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), Error> {
        if data.len() < 16 {
            return Err(Error::Invalid);
        }

        let size = u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
        if size != Self::pod_size() {
            return Err(Error::Invalid);
        }

        let t = u32::from_ne_bytes(data[4..8].try_into().unwrap());
        if t != Self::pod_type() as u32 {
            return Err(Error::Invalid);
        }

        let val = Self::decode_body(&data[8..])?;
        let padding = pad_8(size);
        Ok((val, 8 + size + padding))
    }
}

impl Primitive for () {
    fn pod_type() -> Type {
        Type::None
    }

    fn pod_size() -> usize {
        0
    }

    fn encode_body(&self, _data: &mut [u8]) -> Result<(), Error> {
        Ok(())
    }

    fn decode_body(_data: &[u8]) -> Result<Self, Error> {
        Ok(())
    }
}

impl Primitive for bool {
    fn pod_type() -> Type {
        Type::Bool
    }

    fn pod_size() -> usize {
        4
    }

    fn encode_body(&self, data: &mut [u8]) -> Result<(), Error> {
        data[0..4].copy_from_slice(&(*self as u32).to_ne_bytes());
        Ok(())
    }

    fn decode_body(data: &[u8]) -> Result<Self, Error> {
        let val = u32::from_ne_bytes(data[0..4].try_into().unwrap()) != 0;
        Ok(val)
    }
}

impl<T> Primitive for Id<T>
where
    T: Into<u32> + TryFrom<u32> + Copy,
{
    fn pod_type() -> Type {
        Type::Id
    }

    fn pod_size() -> usize {
        4
    }

    fn encode_body(&self, data: &mut [u8]) -> Result<(), Error> {
        data[0..4].copy_from_slice(&self.0.into().to_ne_bytes());
        Ok(())
    }

    fn decode_body(data: &[u8]) -> Result<Self, Error> {
        let raw_val = u32::from_ne_bytes(data[0..4].try_into().unwrap());
        if let Ok(val) = raw_val.try_into() {
            Ok(Id(val))
        } else {
            Err(Error::Invalid)
        }
    }
}

impl Primitive for i32 {
    fn pod_type() -> Type {
        Type::Int
    }

    fn pod_size() -> usize {
        4
    }

    fn encode_body(&self, data: &mut [u8]) -> Result<(), Error> {
        data[0..4].copy_from_slice(&self.to_ne_bytes());
        Ok(())
    }

    fn decode_body(data: &[u8]) -> Result<Self, Error> {
        let val = i32::from_ne_bytes(data[0..4].try_into().unwrap());
        Ok(val)
    }
}

impl Primitive for i64 {
    fn pod_type() -> Type {
        Type::Long
    }

    fn pod_size() -> usize {
        8
    }

    fn encode_body(&self, data: &mut [u8]) -> Result<(), Error> {
        data[0..8].copy_from_slice(&self.to_ne_bytes());
        Ok(())
    }

    fn decode_body(data: &[u8]) -> Result<Self, Error> {
        let val = i64::from_ne_bytes(data[0..8].try_into().unwrap());
        Ok(val)
    }
}

impl Primitive for f32 {
    fn pod_type() -> Type {
        Type::Float
    }

    fn pod_size() -> usize {
        4
    }

    fn encode_body(&self, data: &mut [u8]) -> Result<(), Error> {
        data[0..4].copy_from_slice(&self.to_ne_bytes());
        Ok(())
    }

    fn decode_body(data: &[u8]) -> Result<Self, Error> {
        let val = f32::from_ne_bytes(data[0..4].try_into().unwrap());
        Ok(val)
    }
}

impl Primitive for f64 {
    fn pod_type() -> Type {
        Type::Double
    }

    fn pod_size() -> usize {
        8
    }

    fn encode_body(&self, data: &mut [u8]) -> Result<(), Error> {
        data[0..8].copy_from_slice(&self.to_ne_bytes());
        Ok(())
    }

    fn decode_body(data: &[u8]) -> Result<Self, Error> {
        let val = f64::from_ne_bytes(data[0..8].try_into().unwrap());
        Ok(val)
    }
}

impl Primitive for Fd {
    fn pod_type() -> Type {
        Type::Fd
    }

    fn pod_size() -> usize {
        8
    }

    fn encode_body(&self, data: &mut [u8]) -> Result<(), Error> {
        data[0..8].copy_from_slice(&(self.0 as i64).to_ne_bytes());
        Ok(())
    }

    fn decode_body(data: &[u8]) -> Result<Fd, Error> {
        let val = i64::from_ne_bytes(data[0..8].try_into().unwrap());
        Ok(Fd(val as RawFd))
    }
}

impl Primitive for Rectangle {
    fn pod_type() -> Type {
        Type::Rectangle
    }

    fn pod_size() -> usize {
        8
    }

    fn encode_body(&self, data: &mut [u8]) -> Result<(), Error> {
        data[0..4].copy_from_slice(&self.width.to_ne_bytes());
        data[4..8].copy_from_slice(&self.height.to_ne_bytes());
        Ok(())
    }

    fn decode_body(data: &[u8]) -> Result<Rectangle, Error> {
        let width = u32::from_ne_bytes(data[0..4].try_into().unwrap());
        let height = u32::from_ne_bytes(data[4..8].try_into().unwrap());

        Ok(Rectangle { width, height })
    }
}

impl Primitive for Fraction {
    fn pod_type() -> Type {
        Type::Fraction
    }

    fn pod_size() -> usize {
        8
    }

    fn encode_body(&self, data: &mut [u8]) -> Result<(), Error> {
        data[0..4].copy_from_slice(&self.num.to_ne_bytes());
        data[4..8].copy_from_slice(&self.denom.to_ne_bytes());
        Ok(())
    }

    fn decode_body(data: &[u8]) -> Result<Fraction, Error> {
        let num = u32::from_ne_bytes(data[0..4].try_into().unwrap());
        let denom = u32::from_ne_bytes(data[4..8].try_into().unwrap());

        Ok(Fraction { num, denom })
    }
}

impl Pod for &str {
    type DecodesTo = String;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error> {
        let len = self.len() + 1;
        let padding = pad_8(len);

        if len as u32 > u32::MAX || data.len() < 8 + len + padding {
            return Err(Error::NoSpace);
        }

        data[0..4].copy_from_slice(&(len as u32).to_ne_bytes());
        data[4..8].copy_from_slice(&(Type::String as u32).to_ne_bytes());
        data[8..8 + self.len()].copy_from_slice(self.as_bytes());
        // Null terminator
        data[8 + self.len()] = 0;
        // Padding
        data[8 + len..8 + len + padding].copy_from_slice(&[0; 8][0..padding]);

        Ok(8 + len + padding)
    }

    fn decode(data: &[u8]) -> Result<(String, usize), Error> {
        let len = u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
        let padding = pad_8(len);

        if data.len() < 8 + len {
            return Err(Error::Invalid);
        }

        if data[4..8] != (Type::String as u32).to_ne_bytes() {
            return Err(Error::Invalid);
        }

        let s = String::from_utf8_lossy(&data[8..8 + len - 1]).to_string();
        // Null terminator
        if data[8 + len - 1] != 0 {
            return Err(Error::Invalid);
        }

        Ok((s, 8 + len + padding))
    }
}

impl Pod for String {
    type DecodesTo = String;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error> {
        self.as_str().encode(data)
    }

    fn decode(data: &[u8]) -> Result<(String, usize), Error> {
        // &str also decodes to String
        <&str as Pod>::decode(data)
    }
}

impl Pod for &[u8] {
    type DecodesTo = Vec<u8>;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error> {
        let len = self.len();
        let padding = pad_8(len);

        if len as u32 > u32::MAX || data.len() < 8 + len + padding {
            return Err(Error::NoSpace);
        }

        data[0..4].copy_from_slice(&(len as u32).to_ne_bytes());
        data[4..8].copy_from_slice(&(Type::Bytes as u32).to_ne_bytes());
        data[8..8 + self.len()].copy_from_slice(self);
        // Padding
        data[8 + len..8 + len + padding].copy_from_slice(&[0; 8][0..padding]);

        Ok(8 + len + padding)
    }

    fn decode(data: &[u8]) -> Result<(Vec<u8>, usize), Error> {
        let len = u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
        let padding = pad_8(len);

        if data.len() < 8 + len {
            return Err(Error::Invalid);
        }

        if data[4..8] != (Type::Bytes as u32).to_ne_bytes() {
            return Err(Error::Invalid);
        }

        Ok((data[8..8 + len].to_vec(), 8 + len + padding))
    }
}

impl Pod for Vec<u8> {
    type DecodesTo = Vec<u8>;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error> {
        self.as_slice().encode(data)
    }

    fn decode(data: &[u8]) -> Result<(Vec<u8>, usize), Error> {
        // &[u8] also decodes to Vec<u8>
        <&[u8] as Pod>::decode(data)
    }
}

// Pointer is encoded as:
//
// +--------------+
// |  total size  | 4 bytes
// +--------------+
// |   pod type   | 4 bytes
// +--------------+
// |   ptr size   | 4 bytes
// +--------------+
// | pointee type | 4 bytes
// +--------------+
// | pointer val  | 4 or 8 bytes
// +--------------+
// |   padding?   | 4 bytes
// +--------------+
//
impl Pod for Pointer {
    type DecodesTo = Pointer;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error> {
        let ptr_size = std::mem::size_of::<*const c_void>();
        let size = 4 /* type */ + 4 /* _padding */ + ptr_size /* pointer */;

        // size + type + type_of_ptr + 4 (padding_) + ptr + (maybe padding for u32ptr)
        if data.len() < 24 {
            return Err(Error::NoSpace);
        }

        data[0..4].copy_from_slice(&(size as u32).to_ne_bytes());
        data[4..8].copy_from_slice(&(Type::Pointer as u32).to_ne_bytes());
        data[8..12].copy_from_slice(&(self.type_ as u32).to_ne_bytes());
        data[12..16].copy_from_slice(&[0, 0, 0, 0]);
        if ptr_size == 8 {
            data[16..24].copy_from_slice(&(self.ptr as u64).to_ne_bytes());
        } else {
            data[16..20].copy_from_slice(&(self.ptr as u32).to_ne_bytes());
            data[20..24].copy_from_slice(&[0, 0, 0, 0]);
        }

        Ok(24)
    }

    fn decode(data: &[u8]) -> Result<(Pointer, usize), Error> {
        let size = u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
        let ptr_size = std::mem::size_of::<*const c_void>();
        let padding = 8 - ptr_size;

        if data.len() < 24 {
            return Err(Error::Invalid);
        }

        if data[4..8] != (Type::Pointer as u32).to_ne_bytes() {
            return Err(Error::Invalid);
        }

        let type_ = match u32::from_ne_bytes(data[8..12].try_into().unwrap()).try_into() {
            Ok(t) => t,
            Err(_) => return Err(Error::Invalid),
        };
        let ptr = if ptr_size == 8 {
            u64::from_ne_bytes(data[16..24].try_into().unwrap()) as *const c_void
        } else {
            u32::from_ne_bytes(data[16..20].try_into().unwrap()) as *const c_void
        };

        Ok((Pointer { type_, ptr }, 8 + size + padding))
    }
}

// Array is encoded as:
//
// +--------------+
// |  total size  | 4 bytes
// +--------------+
// |   pod type   | 4 bytes
// +--------------+
// | 1 child size | 4 bytes
// +--------------+
// |  child type  | 4 bytes
// +--------------+
// |  elem data   |
// |    bodies    |
// +--------------+
// |   padding?   | 4 bytes
// +--------------+
//
impl<T> Pod for &[T]
where
    T: Primitive,
{
    type DecodesTo = Vec<T>;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error> {
        let child_size = T::pod_size();
        let child_type = T::pod_type();
        let elems_size = child_size * self.len();
        let padding = pad_8(elems_size);

        if data.len() < 8 + 8 + elems_size + padding {
            return Err(Error::NoSpace);
        }

        data[0..4].copy_from_slice(&(8 + elems_size as u32).to_ne_bytes());
        data[4..8].copy_from_slice(&(Type::Array as u32).to_ne_bytes());
        data[8..12].copy_from_slice(&(child_size as u32).to_ne_bytes());
        data[12..16].copy_from_slice(&(child_type as u32).to_ne_bytes());

        for i in 0..self.len() {
            self[i].encode_body(&mut data[16 + i * child_size..])?;
        }

        Ok(8 + 8 + elems_size + padding)
    }

    fn decode(data: &[u8]) -> Result<(Vec<T>, usize), Error> {
        let mut res = Vec::new();

        if data.len() < 16 {
            return Err(Error::Invalid);
        }

        let size = u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
        let padding = pad_8(size);

        if data.len() < 8 + size + padding {
            return Err(Error::Invalid);
        }

        if Ok(Type::Array) != u32::from_ne_bytes(data[4..8].try_into().unwrap()).try_into() {
            return Err(Error::Invalid);
        }

        let child_size = u32::from_ne_bytes(data[8..12].try_into().unwrap()) as usize;
        if Ok(T::pod_type()) != u32::from_ne_bytes(data[12..16].try_into().unwrap()).try_into() {
            return Err(Error::Invalid);
        }

        for i in 0..(size - 8) / child_size {
            let val = T::decode_body(&data[16 + i * child_size..])?;
            res.push(val);
        }

        Ok((res, 8 + size + padding))
    }
}

// Same as &[T]
impl<T> Pod for Vec<T>
where
    T: Primitive,
{
    type DecodesTo = Self;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error> {
        self.as_slice().encode(data)
    }

    fn decode(data: &[u8]) -> Result<(Vec<T>, usize), Error> {
        <&[T]>::decode(data)
    }
}

// Choice is encoded as:
//
// +--------------+
// |  total size* | 4 bytes
// +--------------+
// |   pod type   | 4 bytes
// +--------------+
// | choice type  | 4 bytes
// +--------------+
// |    flags     | 4 bytes
// +--------------+
// |  child size  | 4 bytes
// +--------------+
// |  child type  | 4 bytes
// +--------------+
// |  elem data   |
// |    bodies    |
// +--------------+
// |   padding?   | 4 bytes
// +--------------+
//
impl<T> Pod for Choice<T>
where
    T: Pod + Primitive,
{
    type DecodesTo = Choice<T>;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error> {
        let child_size = T::pod_size();
        let size = 16
            + match self {
                Choice::None(_) => child_size,
                Choice::Range { .. } => child_size * 3,
                Choice::Step { .. } => child_size * 4,
                Choice::Enum {
                    default: _,
                    alternatives,
                } => child_size * (1 + alternatives.len()),
                Choice::Flags { .. } => child_size * 2,
            };
        let padding = pad_8(size);

        if data.len() < 24 + size + padding {
            return Err(Error::NoSpace);
        }

        let choice_type = match self {
            Choice::None(_) => 0u32,
            Choice::Range { .. } => 1,
            Choice::Step { .. } => 2,
            Choice::Enum { .. } => 3,
            Choice::Flags { .. } => 4,
        };

        data[0..4].copy_from_slice(&(size as u32).to_ne_bytes());
        data[4..8].copy_from_slice(&(Type::Choice as u32).to_ne_bytes());
        data[8..12].copy_from_slice(&choice_type.to_ne_bytes());
        // flags is unused, so we don't bother exposing it
        data[12..16].copy_from_slice(&0u32.to_ne_bytes());
        data[16..20].copy_from_slice(&(T::pod_size() as u32).to_ne_bytes());
        data[20..24].copy_from_slice(&(T::pod_type() as u32).to_ne_bytes());

        match self {
            Choice::None(value) => {
                value.encode_body(&mut data[24..])?;
            }
            Choice::Range { default, min, max } => {
                default.encode_body(&mut data[24..])?;
                min.encode_body(&mut data[24 + child_size..])?;
                max.encode_body(&mut data[24 + child_size * 2..])?;
            }
            Choice::Step {
                default,
                min,
                max,
                step,
            } => {
                default.encode_body(&mut data[24..])?;
                min.encode_body(&mut data[24 + child_size..])?;
                max.encode_body(&mut data[24 + child_size * 2..])?;
                step.encode_body(&mut data[24 + child_size * 3..])?;
            }
            Choice::Enum {
                default,
                alternatives,
            } => {
                default.encode_body(&mut data[24..])?;
                for (i, alt) in alternatives.iter().enumerate() {
                    alt.encode_body(&mut data[24 + child_size * (i + 1)..])?;
                }
            }
            Choice::Flags { default, flags } => {
                default.encode_body(&mut data[24..])?;
                flags.encode_body(&mut data[24 + child_size..])?;
            }
        }

        Ok(8 + size + padding)
    }

    fn decode(data: &[u8]) -> Result<(Choice<T>, usize), Error> {
        if data.len() < 24 {
            return Err(Error::Invalid);
        }

        let size = u32::from_ne_bytes(data[0..4].try_into().unwrap()) as usize;
        let padding = pad_8(size);

        if data.len() < 8 + size + padding {
            return Err(Error::Invalid);
        }

        if u32::from_ne_bytes(data[4..8].try_into().unwrap()) != Type::Choice as u32 {
            return Err(Error::Invalid);
        }

        let choice_type = u32::from_ne_bytes(data[8..12].try_into().unwrap());
        // flags is unused, so we don't decode it at [12..16]
        let child_size = u32::from_ne_bytes(data[16..20].try_into().unwrap()) as usize;
        if child_size != T::pod_size() {
            return Err(Error::Invalid);
        }
        let child_type = u32::from_ne_bytes(data[20..24].try_into().unwrap());
        if child_type != T::pod_type() as u32 {
            return Err(Error::Invalid);
        }

        let choice = match choice_type {
            0 => {
                let value = T::decode_body(&data[24..])?;
                Choice::None(value)
            }
            1 => {
                if size != 16 + child_size * 3 {
                    return Err(Error::Invalid);
                }

                let default = T::decode_body(&data[24..])?;
                let min = T::decode_body(&data[24 + child_size..])?;
                let max = T::decode_body(&data[24 + child_size * 2..])?;

                Choice::Range { default, min, max }
            }
            2 => {
                if size != 16 + child_size * 4 {
                    return Err(Error::Invalid);
                }

                let default = T::decode_body(&data[24..])?;
                let min = T::decode_body(&data[24 + child_size..])?;
                let max = T::decode_body(&data[24 + child_size * 2..])?;
                let step = T::decode_body(&data[24 + child_size * 3..])?;

                Choice::Step {
                    default,
                    min,
                    max,
                    step,
                }
            }
            3 => {
                let default = T::decode_body(&data[24..])?;
                let mut alternatives = Vec::new();

                for i in 1..(size - 16) / child_size {
                    alternatives.push(T::decode_body(&data[24 + child_size * i..])?);
                }

                Choice::Enum {
                    default,
                    alternatives,
                }
            }
            4 => {
                if size != 16 + child_size * 2 {
                    return Err(Error::Invalid);
                }

                let default = T::decode_body(&data[24..])?;
                let flags = T::decode_body(&data[24 + child_size..])?;

                Choice::Flags { default, flags }
            }
            _ => return Err(Error::Invalid),
        };

        Ok((choice, 8 + size + padding))
    }
}

impl<T, U> Pod for Property<T, U>
where
    T: Copy + Into<u32> + TryFrom<u32>,
    U: Pod,
{
    type DecodesTo = Property<T, U::DecodesTo>;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error> {
        if data.len() < 8 {
            return Err(Error::NoSpace);
        }

        data[0..4].copy_from_slice(&self.key.into().to_ne_bytes());
        data[4..8].copy_from_slice(&self.flags.bits().to_ne_bytes());

        let value_size = self.value.encode(&mut data[8..])?;

        Ok(8 + value_size)
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), Error> {
        if data.len() < 8 {
            return Err(Error::Invalid);
        }

        let key = match T::try_from(u32::from_ne_bytes(data[0..4].try_into().unwrap())) {
            Ok(k) => k,
            Err(_) => return Err(Error::Invalid),
        };

        let flags =
            match PropertyFlags::from_bits(u32::from_ne_bytes(data[4..8].try_into().unwrap())) {
                Some(f) => f,
                None => return Err(Error::Invalid),
            };

        let (value, size) = U::decode(&data[8..])?;

        Ok((Property { key, flags, value }, 8 + size))
    }
}

impl<T: Pod> Pod for Option<T> {
    type DecodesTo = Option<T::DecodesTo>;

    fn encode(&self, data: &mut [u8]) -> Result<usize, Error> {
        match self {
            Some(v) => v.encode(data),
            None => ().encode(data),
        }
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), Error> {
        let res = T::decode(data);

        match res {
            Ok((v, size)) => Ok((Some(v), size)),
            Err(_) => {
                let (_, size) = <()>::decode(data)?;
                Ok((None, size))
            }
        }
    }
}
