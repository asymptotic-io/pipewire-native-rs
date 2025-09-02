// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use super::{
    parser, types::Type, Choice, Error, Fraction, Id, Pod, Primitive, RawPod, RawPodOwned,
    Rectangle,
};

impl<'a> std::fmt::Debug for RawPod<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RawPod {{ type: {:?}, size: {} }}",
            self.type_, self.size
        )
    }
}

impl<'a> std::fmt::Display for RawPod<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.type_.is_primitive() {
            write_primitive(f, self.type_, &self.data[8..self.size])
        } else {
            match self.type_() {
                Type::String => write!(f, "{}", String::decode(self.data)?.0),
                Type::Bytes => write!(f, "{:?}", Vec::<u8>::decode(self.data)?.0),
                Type::Bitmap => write!(f, "bitmap not implemented"),
                Type::Array => {
                    let mut parser = parser::Parser::new(self.data);

                    write!(f, "[ ")?;
                    parser.pop_array_raw(|type_, data| {
                        write_primitive(f, type_, data)
                            .and_then(|_| write!(f, ", "))
                            .map_err(|_| Error::Invalid("Could not write array member".to_string()))
                    })?;
                    write!(f, " ]")
                }
                Type::Struct => {
                    let mut parser = parser::Parser::new(self.data);

                    write!(f, "{{ ")?;
                    parser.pop_struct(|parser| {
                        while parser.available() > 0 {
                            let pod = parser.pop_raw_pod()?;
                            write!(f, "{pod}, ").map_err(|_| {
                                Error::Invalid("Could not write struct member".to_string())
                            })?;
                        }
                        Ok(())
                    })?;
                    write!(f, " }}")
                }
                Type::Object => {
                    let mut parser = parser::Parser::new(self.data);

                    write!(f, "{{ ")?;
                    parser.pop_object_raw(|object_parser, _type_, _id: u32| {
                        for (id, _, pod) in object_parser {
                            write!(f, "{id}: {pod}, ").map_err(|_| {
                                Error::Invalid("Could not write object member".to_string())
                            })?;
                        }
                        Ok(())
                    })?;
                    write!(f, " }}")
                }
                Type::Sequence => write!(f, "sequence"),
                Type::Pointer => write!(f, "pointer"),
                Type::Choice => {
                    let mut parser = parser::Parser::new(self.data);
                    write!(f, "{{ ")?;
                    parser.pop_choice_raw(|type_, choice| {
                        match choice {
                            Choice::None(v) => write_primitive(f, type_, v),
                            Choice::Range { default, min, max } => write!(f, "default: ")
                                .and_then(|_| write_primitive(f, type_, default))
                                .and_then(|_| write!(f, ", min: "))
                                .and_then(|_| write_primitive(f, type_, min))
                                .and_then(|_| write!(f, ", max: "))
                                .and_then(|_| write_primitive(f, type_, max)),
                            Choice::Step {
                                default,
                                min,
                                max,
                                step,
                            } => write!(f, "default: ")
                                .and_then(|_| write_primitive(f, type_, default))
                                .and_then(|_| write!(f, ", min: "))
                                .and_then(|_| write_primitive(f, type_, min))
                                .and_then(|_| write!(f, ", max: "))
                                .and_then(|_| write_primitive(f, type_, max))
                                .and_then(|_| write!(f, ", step: "))
                                .and_then(|_| write_primitive(f, type_, step)),
                            Choice::Enum {
                                default,
                                alternatives,
                            } => write!(f, "default: ")
                                .and_then(|_| write_primitive(f, type_, default))
                                .and_then(|_| write!(f, ", [ "))
                                .map(|_| {
                                    alternatives
                                        .iter()
                                        .flat_map(|v| {
                                            write_primitive(f, type_, v)
                                                .and_then(|_| write!(f, ", "))
                                        })
                                        .collect::<()>()
                                })
                                .and_then(|_| write!(f, " ]")),
                            Choice::Flags { default, flags } => write!(f, "default: ")
                                .and_then(|_| write_primitive(f, type_, default))
                                .and_then(|_| write!(f, "f, lags: "))
                                .and_then(|_| write_primitive(f, type_, flags)),
                        }
                        .map_err(|_| Error::Invalid("Could not write choice".to_string()))?;
                        Ok(())
                    })?;
                    write!(f, " }}")
                }
                Type::Pod => write!(f, "pod"),
                t => unreachable!("Type {t:?} should not be seen"),
            }
        }
    }
}

fn write_primitive(
    f: &mut std::fmt::Formatter,
    type_: Type,
    data: &[u8],
) -> Result<(), std::fmt::Error> {
    match type_ {
        Type::None => write!(f, "()"),
        Type::Bool => write!(f, "{}", bool::decode_body(data)?),
        Type::Id => write!(f, "Id({})", Id::<u32>::decode_body(data)?.0),
        Type::Int => write!(f, "{}", i32::decode_body(data)?),
        Type::Long => write!(f, "{}", i64::decode_body(data)?),
        Type::Float => write!(f, "{}", f32::decode_body(data)?),
        Type::Double => write!(f, "{}", f64::decode_body(data)?),
        Type::Rectangle => {
            let rect = Rectangle::decode_body(data)?;
            write!(f, "{}x{}", rect.width, rect.height)
        }
        Type::Fraction => {
            let frac = Fraction::decode_body(data)?;
            write!(f, "{}/{}", frac.num, frac.denom)
        }
        Type::Fd => write!(f, "fd"),
        _ => write!(f, "Type {type_:?} is not a primitive"),
    }
}

impl std::fmt::Debug for RawPodOwned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "RawPodOwned {{ type: {:?}, size: {} }}",
            self.type_, self.size
        )
    }
}
