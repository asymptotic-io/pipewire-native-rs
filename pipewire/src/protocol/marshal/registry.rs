// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa::{self as spa, pod::Pod};

use crate::{
    closure, default_topic, hasproxy_method_call, log,
    permission::PermissionBits,
    properties::Properties,
    protocol::connection::Connection,
    proxy::{
        self,
        registry::{Registry, RegistryMethods},
        HasProxy, Proxy,
    },
    proxy_object_notify, trace, types, Id,
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
            bind: closure!([connection] proxy, id, type_, version, {
                let registry = proxy.object().unwrap();
                let core = registry.core();

                let new_object: Box<dyn HasProxy> = match type_ {
                    types::interface::CLIENT => Box::new(proxy::client::Client::new(&core)),
                    types::interface::MODULE => Box::new(proxy::module::Module::new(&core)),
                    _ => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Unsupported proxy type {type_}"),
                        ))
                    }
                };

                connection.push(
                    proxy.id(),
                    Methods::Bind(Bind {
                        id: id as i32,
                        type_: type_.to_string(),
                        version: version as i32,
                        new_id: hasproxy_method_call!(new_object, id) as i32,
                    }),
                )?;

                Ok(new_object)
            }),
            destroy: closure!([connection] proxy, id, {
                connection.push(proxy.id(), Methods::Destroy(Destroy { id: id as i32 }))
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
        header: &super::message::Header,
        proxy: Proxy<Registry>,
    ) -> std::io::Result<()> {
        let event = connection.decode_core_message::<Events>(header)?;

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
