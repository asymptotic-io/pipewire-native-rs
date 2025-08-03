// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa as spa;

use crate::{
    closure,
    core::{Core, CoreMethods},
    protocol::connection::Connection,
};

#[derive(macros::PodStruct)]
struct Hello {
    version: i64,
}

pub(crate) fn methods(connection: Connection) -> CoreMethods<Core> {
    CoreMethods {
        hello: closure!(connection, proxy, version, {
            connection.push(
                proxy.id(),
                0,
                Hello {
                    version: version as i64,
                },
            )
        }),
        sync: closure!(connection, proxy, seq, { todo!() }),
        pong: closure!(connection, proxy, seq, { todo!() }),
        error: closure!(connection, proxy, seq, res, message, { todo!() }),
        create_object: closure!(connection, proxy, factory_name, type_, version, props, {
            todo!()
        }),
        destroy: closure!(connection, proxy, object, { todo!() }),
    }
}
