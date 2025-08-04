// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa::{self as spa, pod::Pod};

use crate::{
    closure,
    core::{Core, CoreMethods},
    protocol::{connection::Connection, ASYNC_SEQ_BIT, ASYNC_SEQ_MASK},
};

use super::Marshallable;

enum Methods {
    Hello(Hello),
    Sync(Sync),
    Pong(Pong),
    Error(()),
    GetRegistry(()),
    CreateObject(()),
    Destroy(()),
}

impl Marshallable for Methods {
    fn opcode(&self) -> u8 {
        match self {
            Self::Hello(_) => 1,
            Self::Sync(_) => 2,
            Self::Pong(_) => 3,
            Self::Error(_) => 4,
            Self::GetRegistry(_) => 5,
            Self::CreateObject(_) => 6,
            Self::Destroy(_) => 7,
        }
    }
    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error> {
        match self {
            Self::Hello(o) => o.encode(data),
            Self::Sync(o) => o.encode(data),
            Self::Pong(o) => o.encode(data),
            _ => todo!(),
        }
    }

    fn decode(opcode: u8, data: &[u8]) -> Result<(Self, usize), spa::pod::Error> {
        match opcode {
            1 => Hello::decode(data).map(|(o, s)| (Self::Hello(o), s)),
            2 => Sync::decode(data).map(|(o, s)| (Self::Sync(o), s)),
            3 => Pong::decode(data).map(|(o, s)| (Self::Pong(o), s)),
            _ => todo!(),
        }
    }
}

#[derive(macros::PodStruct)]
struct Hello {
    version: i64,
}

#[derive(macros::PodStruct)]
struct Sync {
    id: i64,
    seq: i64,
}

#[derive(macros::PodStruct)]
struct Pong {
    id: i64,
    seq: i64,
}

pub(crate) fn methods(connection: Connection) -> CoreMethods<Core> {
    CoreMethods {
        hello: closure!(connection, proxy, version, {
            connection.push(
                proxy.id(),
                Methods::Hello(Hello {
                    version: version as i64,
                }),
            )
        }),
        sync: closure!(connection, proxy, id, {
            let seq = ASYNC_SEQ_BIT | (connection.next_seq() & ASYNC_SEQ_MASK);
            connection.push(
                proxy.id(),
                Methods::Sync(Sync {
                    id: id as i64,
                    seq: seq as i64,
                }),
            )
        }),
        pong: closure!(connection, proxy, id, seq, {
            connection.push(
                proxy.id(),
                Methods::Pong(Pong {
                    id: id as i64,
                    seq: seq as i64,
                }),
            )
        }),
        error: closure!(connection, proxy, seq, res, message, { todo!() }),
        create_object: closure!(connection, proxy, factory_name, type_, version, props, {
            todo!()
        }),
        destroy: closure!(connection, proxy, object, { todo!() }),
    }
}
