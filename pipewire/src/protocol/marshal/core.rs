// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa as spa;

use crate::{
    closure,
    core::{Core, CoreMethods},
    protocol::{connection::Connection, ASYNC_SEQ_BIT, ASYNC_SEQ_MASK},
};

#[repr(u8)]
enum Methods {
    Hello = 1,
    Sync,
    Pong,
    Error,
    GetRegistry,
    CreateObject,
    Destroy,
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
                Methods::Hello as u8,
                Hello {
                    version: version as i64,
                },
            )
        }),
        sync: closure!(connection, proxy, id, {
            let seq = ASYNC_SEQ_BIT | (connection.next_seq() & ASYNC_SEQ_MASK);
            connection.push(
                proxy.id(),
                Methods::Sync as u8,
                Sync {
                    id: id as i64,
                    seq: seq as i64,
                },
            )
        }),
        pong: closure!(connection, proxy, id, seq, {
            connection.push(
                proxy.id(),
                Methods::Pong as u8,
                Pong {
                    id: id as i64,
                    seq: seq as i64,
                },
            )
        }),
        error: closure!(connection, proxy, seq, res, message, { todo!() }),
        create_object: closure!(connection, proxy, factory_name, type_, version, props, {
            todo!()
        }),
        destroy: closure!(connection, proxy, object, { todo!() }),
    }
}
