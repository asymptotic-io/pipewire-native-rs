// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa::{self as spa, pod::Pod};

use crate::{
    closure, default_topic, log,
    permission::PermissionBits,
    properties::Properties,
    protocol::connection::Connection,
    proxy::{
        registry::{Registry, RegistryMethods},
        Proxy,
    },
    proxy_object_notify, trace, Id,
};

use super::PairList;

default_topic!(log::topic::PROTOCOL);

#[repr(u8)]
#[derive(Debug, macros::Marshallable)]
pub(crate) enum Methods {
    Bind(Bind) = 1,
    Destroy(Destroy),
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Bind {
    id: i32,
    type_: String,
    version: i32,
    new_id: i32,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Destroy {
    id: i32,
}

impl Methods {
    pub(crate) fn marshal(connection: Connection) -> RegistryMethods<Registry> {
        RegistryMethods {
            bind: closure!(_connection <- connection, _proxy, _id, _type_, _version, {
                // TODO: create a proxy of type_
                // TODO: send a Registry::Bind method
                todo!()
            }),
            destroy: closure!(_connection <- connection, _proxy, _id, {
                // TODO: destroy the proxy?
                todo!()
            }),
        }
    }
}

#[derive(Debug, macros::Marshallable)]
pub(crate) enum Events {
    Global(Global),
    GlobalRemove(GlobalRemove),
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Global {
    id: i32,
    permissions: i32,
    type_: String,
    verison: i32,
    props: PairList<String, String>,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct GlobalRemove {
    id: i32,
}

impl Events {
    pub(crate) fn demarshal(
        connection: &Connection,
        header: &super::Header,
        proxy: Proxy<Registry>,
    ) -> std::io::Result<()> {
        let event = connection.decode_message::<Events>(header)?;

        trace!("got event: {event:?}");

        match event {
            Events::Global(global) => {
                let props = Properties::new_vec(global.props.data);
                proxy_object_notify!(
                    proxy,
                    global,
                    global.id as Id,
                    PermissionBits::from_bits_truncate(global.permissions as u32),
                    &global.type_,
                    global.verison as u32,
                    &props
                )
            }
            Events::GlobalRemove(remove) => {
                proxy_object_notify!(proxy, global_remove, remove.id as Id)
            }
        }

        Ok(())
    }
}
