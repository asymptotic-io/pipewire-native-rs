// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::ffi::c_void;

use pipewire_native_spa::pod::builder::Builder;
use pipewire_native_spa::pod::parser::Parser;
use pipewire_native_spa::pod::types::{
    Choice, Fd, Fraction, Id, ObjectType, Pointer, Property, PropertyFlags, Rectangle, Type,
};
use pipewire_native_spa::pod::Pod;
use pipewire_native_spa::types::params::{ParamType, PropInfo};

use libspa::pod as spa_pod;
use libspa::sys::{self as spa_sys};
use libspa::utils as spa_utils;

#[test]
fn test_pod_builder() {
    let mut buf = [0u8; 1024];
    let builder = Builder::new(&mut buf);
    let res = builder
        .push_none()
        .push_bool(true)
        .push_id(Id(1u32))
        .push_int(2)
        .push_long(3)
        .push_float(4.0)
        .push_double(5.0)
        .push_string("hello")
        .push_bytes(&[6, 7, 8, 9])
        .push_pointer(Type::Int, 0xdeadc0de as *const c_void)
        .push_fd(-1)
        .push_rectangle(1920, 1080)
        .push_fraction(30001, 1)
        .push_array(&[11.0f32, 12.0, 13.0])
        .push_array::<bool>(&[])
        .push_choice(Choice::None(14i64))
        .push_choice(Choice::Range {
            default: 1i32,
            min: 0,
            max: 10,
        })
        .push_choice(Choice::Step {
            default: 1.5f32,
            min: 0.0,
            max: 10.0,
            step: 0.25,
        })
        .push_choice(Choice::Enum {
            default: Id(2u32),
            alternatives: [Id(1), Id(2), Id(3), Id(4)].to_vec(),
        })
        .build()
        .unwrap();

    let mut sbuf = Vec::with_capacity(1024);
    let mut sbuilder = spa_pod::builder::Builder::new(&mut sbuf);
    sbuilder.add_none().unwrap();
    sbuilder.add_bool(true).unwrap();
    sbuilder.add_id(spa_utils::Id(1)).unwrap();
    sbuilder.add_int(2).unwrap();
    sbuilder.add_long(3).unwrap();
    sbuilder.add_float(4.0).unwrap();
    sbuilder.add_double(5.0).unwrap();
    sbuilder.add_string("hello").unwrap();
    sbuilder.add_bytes(&[6, 7, 8, 9]).unwrap();
    unsafe {
        sbuilder
            .add_pointer(
                spa_utils::Id(spa_sys::SPA_TYPE_Int),
                0xdeadc0de as *const c_void,
            )
            .unwrap();
    }
    sbuilder.add_fd(-1).unwrap();
    sbuilder
        .add_rectangle(spa_utils::Rectangle {
            width: 1920,
            height: 1080,
        })
        .unwrap();
    sbuilder
        .add_fraction(spa_utils::Fraction {
            num: 30001,
            denom: 1,
        })
        .unwrap();
    unsafe {
        sbuilder
            .add_array(
                4,
                spa_sys::SPA_TYPE_Float,
                3,
                [11.0f32, 12.0, 13.0].as_ptr() as *const c_void,
            )
            .unwrap();
    }
    unsafe {
        sbuilder
            .add_array(4, spa_sys::SPA_TYPE_Bool, 0, [].as_ptr() as *const c_void)
            .unwrap();
    }
    unsafe {
        let mut frame: std::mem::MaybeUninit<spa_sys::spa_pod_frame> =
            std::mem::MaybeUninit::uninit();
        sbuilder
            .push_choice(&mut frame, spa_sys::SPA_CHOICE_None, 0)
            .unwrap();
        sbuilder.add_long(14i64).unwrap();
        sbuilder.pop(&mut frame.assume_init());
    };
    unsafe {
        let mut frame: std::mem::MaybeUninit<spa_sys::spa_pod_frame> =
            std::mem::MaybeUninit::uninit();
        sbuilder
            .push_choice(&mut frame, spa_sys::SPA_CHOICE_Range, 0)
            .unwrap();
        sbuilder.add_int(1).unwrap();
        sbuilder.add_int(0).unwrap();
        sbuilder.add_int(10).unwrap();
        sbuilder.pop(&mut frame.assume_init());
    };
    unsafe {
        let mut frame: std::mem::MaybeUninit<spa_sys::spa_pod_frame> =
            std::mem::MaybeUninit::uninit();
        sbuilder
            .push_choice(&mut frame, spa_sys::SPA_CHOICE_Step, 0)
            .unwrap();
        sbuilder.add_float(1.5).unwrap();
        sbuilder.add_float(0.0).unwrap();
        sbuilder.add_float(10.0).unwrap();
        sbuilder.add_float(0.25).unwrap();
        sbuilder.pop(&mut frame.assume_init());
    };
    unsafe {
        let mut frame: std::mem::MaybeUninit<spa_sys::spa_pod_frame> =
            std::mem::MaybeUninit::uninit();
        sbuilder
            .push_choice(&mut frame, spa_sys::SPA_CHOICE_Enum, 0)
            .unwrap();
        sbuilder.add_id(spa_utils::Id(2)).unwrap();
        sbuilder.add_id(spa_utils::Id(1)).unwrap();
        sbuilder.add_id(spa_utils::Id(2)).unwrap();
        sbuilder.add_id(spa_utils::Id(3)).unwrap();
        sbuilder.add_id(spa_utils::Id(4)).unwrap();
        sbuilder.pop(&mut frame.assume_init());
    };

    assert_eq!(res, sbuf.as_slice());
}

fn test_a_pod<T: Clone + Pod>(pod: &T)
where
    <T as Pod>::DecodesTo: From<T> + std::cmp::PartialEq + std::fmt::Debug,
{
    let mut buf = [0u8; 1024];

    let size = pod.encode(&mut buf).unwrap();
    let (rv, rsize) = T::decode(&buf).unwrap();

    assert_eq!(size, rsize);
    assert_eq!(<T as Pod>::DecodesTo::from(pod.clone()), rv);
}

#[test]
fn test_pod_decode() {
    test_a_pod(&());
    test_a_pod(&true);
    test_a_pod(&(-123 as i32));
    test_a_pod(&(i64::MIN));
    test_a_pod(&"hello");
    test_a_pod(&vec![1u8, 2, 3, 4].as_slice());
    test_a_pod(&Pointer {
        type_: Type::Int,
        ptr: 0xdeadbeef as *const c_void,
    });
    test_a_pod(&Fd(-1));
    test_a_pod(&Rectangle {
        width: 1920,
        height: 1080,
    });
    test_a_pod(&Fraction {
        num: 30001,
        denom: 1,
    });
    test_a_pod(&vec![11.0f32, 12.0, 13.0].as_slice());
    test_a_pod(&Choice::None(14i64));
    test_a_pod(&Choice::Range {
        default: 1i32,
        min: 0,
        max: 10,
    });
    test_a_pod(&Choice::Step {
        default: 1.5f32,
        min: 0.0,
        max: 10.0,
        step: 0.25,
    });
    test_a_pod(&Choice::Enum {
        default: Id(2u32),
        alternatives: [Id(1), Id(2), Id(3), Id(4)].to_vec(),
    });
}

#[test]
fn test_pod_parser() {
    let mut buf = [0u8; 1024];
    let builder = Builder::new(&mut buf);
    let res = builder
        .push_none()
        .push_bool(true)
        .push_id(Id(1u32))
        .push_int(2)
        .push_long(3)
        .push_float(4.0)
        .push_double(5.0)
        .push_string("hello")
        .push_bytes(&[6, 7, 8, 9])
        .push_pointer(Type::Int, 0xdeadc0de as *const c_void)
        .push_fd(-1)
        .push_rectangle(1920, 1080)
        .push_fraction(30001, 1)
        .push_array(&[11.0f32, 12.0, 13.0])
        .push_choice(Choice::None(14i64))
        .push_choice(Choice::Range {
            default: 1i32,
            min: 0,
            max: 10,
        })
        .push_choice(Choice::Step {
            default: 1.5f32,
            min: 0.0,
            max: 10.0,
            step: 0.25,
        })
        .push_choice(Choice::Enum {
            default: Id(2u32),
            alternatives: [Id(1), Id(2), Id(3), Id(4)].to_vec(),
        })
        .build()
        .unwrap();

    let parser = Parser::new(res);
    assert_eq!(parser.pop_none().unwrap(), ());
    assert_eq!(parser.pop_bool().unwrap(), true);
    assert_eq!(parser.pop_id().unwrap(), Id(1u32));
    assert_eq!(parser.pop_int().unwrap(), 2);
    assert_eq!(parser.pop_long().unwrap(), 3);
    assert_eq!(parser.pop_float().unwrap(), 4.0);
    assert_eq!(parser.pop_double().unwrap(), 5.0);
    assert_eq!(parser.pop_string().unwrap(), "hello");
    assert_eq!(parser.pop_bytes().unwrap(), vec![6, 7, 8, 9]);
    assert_eq!(
        parser.pop_pointer().unwrap(),
        Pointer {
            type_: Type::Int,
            ptr: 0xdeadc0de as *const c_void,
        }
    );
    assert_eq!(parser.pop_fd().unwrap(), Fd(-1));
    assert_eq!(
        parser.pop_rectangle().unwrap(),
        Rectangle {
            width: 1920,
            height: 1080,
        }
    );
    assert_eq!(
        parser.pop_fraction().unwrap(),
        Fraction {
            num: 30001,
            denom: 1,
        }
    );
    assert_eq!(
        parser.pop_array::<f32>().unwrap(),
        vec![11.0f32, 12.0, 13.0]
    );
    assert_eq!(parser.pop_choice::<i64>().unwrap(), Choice::None(14i64));
    assert_eq!(
        parser.pop_choice::<i32>().unwrap(),
        Choice::Range {
            default: 1i32,
            min: 0,
            max: 10,
        }
    );
    assert_eq!(
        parser.pop_choice::<f32>().unwrap(),
        Choice::Step {
            default: 1.5f32,
            min: 0.0,
            max: 10.0,
            step: 0.25,
        }
    );
    assert_eq!(
        parser.pop_choice::<Id<u32>>().unwrap(),
        Choice::Enum {
            default: Id(2),
            alternatives: [Id(1), Id(2), Id(3), Id(4)].to_vec(),
        }
    );
}

#[test]
fn test_pod_builder_struct_empty() {
    let mut buf = [0u8; 1024];

    let builder = Builder::new(&mut buf);
    let res = builder.push_struct(|b| b).build().unwrap();

    let mut sbuf = Vec::with_capacity(1024);
    let mut sbuilder = spa_pod::builder::Builder::new(&mut sbuf);
    unsafe {
        let mut frame: std::mem::MaybeUninit<spa_sys::spa_pod_frame> =
            std::mem::MaybeUninit::uninit();
        sbuilder.push_struct(&mut frame).unwrap();
        sbuilder.pop(&mut frame.assume_init());
    };
    assert_eq!(res, sbuf.as_slice());

    let parser = Parser::new(&buf);
    assert_eq!(parser.pop_struct(|_| Ok(())).unwrap(), ());
}

#[test]
fn test_pod_builder_struct() {
    let mut buf = [0u8; 1024];

    let builder = Builder::new(&mut buf);
    let res = builder
        .push_struct(|b| {
            b.push_id(Id(1u32))
                .push_long(2)
                .push_rectangle(3840, 2160)
                .push_float(3.0)
        })
        .build()
        .unwrap();

    let mut sbuf = Vec::with_capacity(1024);
    let mut sbuilder = spa_pod::builder::Builder::new(&mut sbuf);
    unsafe {
        let mut frame: std::mem::MaybeUninit<spa_sys::spa_pod_frame> =
            std::mem::MaybeUninit::uninit();
        sbuilder.push_struct(&mut frame).unwrap();
        sbuilder.add_id(spa_utils::Id(1)).unwrap();
        sbuilder.add_long(2).unwrap();
        sbuilder
            .add_rectangle(spa_utils::Rectangle {
                width: 3840,
                height: 2160,
            })
            .unwrap();
        sbuilder.add_float(3.0).unwrap();
        sbuilder.pop(&mut frame.assume_init());
    };
    assert_eq!(res, sbuf.as_slice());

    let parser = Parser::new(&buf);
    assert_eq!(
        parser
            .pop_struct(|p| {
                assert_eq!(p.pop_id().unwrap(), Id(1u32));
                assert_eq!(p.pop_long().unwrap(), 2);
                assert_eq!(
                    p.pop_rectangle().unwrap(),
                    Rectangle {
                        width: 3840,
                        height: 2160
                    }
                );
                assert_eq!(p.pop_float().unwrap(), 3.0);
                Ok(())
            })
            .unwrap(),
        ()
    );
}

#[test]
fn test_pod_builder_object_empty() {
    let mut buf = [0u8; 1024];

    let builder = Builder::new(&mut buf);
    let res = builder
        .push_object(ObjectType::PropInfo, ParamType::PropInfo, |b| b)
        .build()
        .unwrap();

    let mut sbuf = Vec::with_capacity(1024);
    let mut sbuilder = spa_pod::builder::Builder::new(&mut sbuf);
    unsafe {
        let mut frame: std::mem::MaybeUninit<spa_sys::spa_pod_frame> =
            std::mem::MaybeUninit::uninit();
        sbuilder
            .push_object(
                &mut frame,
                spa_sys::SPA_TYPE_OBJECT_PropInfo,
                spa_sys::SPA_PARAM_PropInfo,
            )
            .unwrap();
        sbuilder.pop(&mut frame.assume_init());
    };
    assert_eq!(res, sbuf.as_slice());

    let parser = Parser::new(&buf);
    assert_eq!(
        parser
            .pop_object(|_p, type_, id: ParamType| {
                assert_eq!(type_, ObjectType::PropInfo);
                assert_eq!(id, ParamType::PropInfo);
                Ok(())
            })
            .unwrap(),
        ()
    );
}

#[test]
fn test_pod_builder_object() {
    let mut buf = [0u8; 1024];

    let builder = Builder::new(&mut buf);
    let res = builder
        .push_object(ObjectType::PropInfo, ParamType::PropInfo, |b| {
            b.push_property(PropInfo::Id, PropertyFlags::empty(), Id(1u32))
                .push_property(PropInfo::Description, PropertyFlags::empty(), "test")
        })
        .build()
        .unwrap();

    let mut sbuf = Vec::with_capacity(1024);
    let mut sbuilder = spa_pod::builder::Builder::new(&mut sbuf);
    unsafe {
        let mut frame: std::mem::MaybeUninit<spa_sys::spa_pod_frame> =
            std::mem::MaybeUninit::uninit();
        sbuilder
            .push_object(
                &mut frame,
                spa_sys::SPA_TYPE_OBJECT_PropInfo,
                spa_sys::SPA_PARAM_PropInfo,
            )
            .unwrap();
        sbuilder.add_prop(spa_sys::SPA_PROP_INFO_id, 0).unwrap();
        sbuilder.add_id(spa_utils::Id(1)).unwrap();
        sbuilder
            .add_prop(spa_sys::SPA_PROP_INFO_description, 0)
            .unwrap();
        sbuilder.add_string("test").unwrap();
        sbuilder.pop(&mut frame.assume_init());
    };
    assert_eq!(res, sbuf.as_slice());

    let parser = Parser::new(&buf);
    assert_eq!(
        parser
            .pop_object(|p, type_, id: ParamType| {
                assert_eq!(type_, ObjectType::PropInfo);
                assert_eq!(id, ParamType::PropInfo);
                assert_eq!(
                    p.pop_property::<PropInfo, Id<u32>>().unwrap(),
                    Property {
                        key: PropInfo::Id,
                        flags: PropertyFlags::empty(),
                        value: Id(1u32)
                    }
                );
                assert_eq!(
                    p.pop_property::<PropInfo, &str>().unwrap(),
                    Property {
                        key: PropInfo::Description,
                        flags: PropertyFlags::empty(),
                        value: "test".to_string()
                    }
                );
                Ok(())
            })
            .unwrap(),
        ()
    );
}
