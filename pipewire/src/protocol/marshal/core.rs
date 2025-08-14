// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa::{self as spa, pod::Pod};

use crate::{
    closure,
    core::{Core, CoreChangeMask, CoreInfo, CoreMethods},
    default_topic, hasproxy_method_call, log,
    properties::Properties,
    protocol::{
        connection::Connection, marshal::message::CoreFooter, ASYNC_SEQ_BIT, ASYNC_SEQ_MASK,
    },
    proxy::{self, HasProxy, Proxy},
    proxy_object_notify, trace, Id,
};

use super::PairList;

default_topic!(log::topic::PROTOCOL);

#[repr(u8)]
#[derive(Debug, macros::Marshallable)]
pub(crate) enum Methods {
    Hello(Hello) = 1,
    Sync(Sync),
    Pong(Pong),
    Error(ErrorMethod),
    GetRegistry(GetRegistry),
    CreateObject(CreateObject),
    Destroy(Destroy),
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Hello {
    version: i32,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Sync {
    id: i32,
    seq: i32,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Pong {
    id: i32,
    seq: i32,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct ErrorMethod {
    id: i32,
    seq: i32,
    res: i32,
    message: String,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct GetRegistry {
    version: i32,
    new_id: i32,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct CreateObject {
    factory_name: String,
    type_: String,
    version: i32,
    props: PairList<String, String>,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Destroy {
    id: i32,
}

impl Methods {
    pub(crate) fn marshal(connection: Connection) -> CoreMethods<Core> {
        CoreMethods {
            hello: closure!([connection] proxy, version, {
                connection.push(
                    proxy.id(),
                    Methods::Hello(Hello {
                        version: version as i32,
                    }),
                )
            }),
            sync: closure!([connection] proxy, id, {
                let seq = ASYNC_SEQ_BIT | (connection.next_seq() & ASYNC_SEQ_MASK);
                connection.push(
                    proxy.id(),
                    Methods::Sync(Sync {
                        id: id as i32,
                        seq: seq as i32,
                    }),
                )
            }),
            pong: closure!([connection] proxy, id, seq, {
                connection.push(
                    proxy.id(),
                    Methods::Pong(Pong {
                        id: id as i32,
                        seq: seq as i32,
                    }),
                )
            }),
            error: closure!([connection] proxy, seq, res, message, {
                connection.push(
                    proxy.id(),
                    Methods::Error(ErrorMethod {
                        id: proxy.id() as i32,
                        seq: seq as i32,
                        res: res as i32,
                        message: message.to_string(),
                    }),
                )
            }),
            get_registry: closure!([connection] proxy, {
                let core = proxy.object().unwrap();
                let registry = proxy::registry::Registry::new(&core);

                connection.push(
                    proxy.id(),
                    Methods::GetRegistry(GetRegistry {
                        version: registry.version() as i32,
                        new_id: registry.proxy().id() as i32,
                    }),
                )?;

                Ok(registry)
            }),
            create_object: closure!([connection] proxy, factory_name, type_, version, props, {
                connection.push(
                    proxy.id(),
                    Methods::CreateObject(CreateObject {
                        factory_name: factory_name.to_string(),
                        type_: type_.to_string(),
                        version: version as i32,
                        props: PairList::new(
                            props
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect(),
                        ),
                    }),
                )
            }),
            destroy: closure!([connection] proxy, object, {
                connection.push(
                    proxy.id(),
                    Methods::Destroy(Destroy {
                        id: hasproxy_method_call!(object, id) as i32,
                    }),
                )
            }),
        }
    }
}

#[derive(Debug, macros::Marshallable)]
pub(crate) enum Events {
    Info(Info),
    Done(Done),
    Ping(Ping),
    Error(ErrorEvent),
    RemoveId(RemoveId),
    BoundId(BoundId),
    AddMem(AddMem),
    RemoveMem(RemoveMem),
    BoundProps(BoundProps),
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Info {
    id: i32,
    cookie: i32,
    user_name: String,
    host_name: String,
    version: String,
    name: String,
    change_mask: i64,
    props: PairList<String, String>,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Done {
    id: i32,
    seq: i32,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Ping {
    id: i32,
    seq: i32,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct ErrorEvent {
    id: i32,
    seq: i32,
    res: i32,
    message: String,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct RemoveId {
    id: i32,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct BoundId {
    id: i32,
    global_id: i32,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct AddMem {
    // TODO
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct RemoveMem {
    // TODO
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct BoundProps {
    id: i32,
    global_id: i32,
    props: PairList<String, String>,
}

impl Events {
    pub(crate) fn demarshal(
        connection: &Connection,
        header: &super::message::Header,
        proxy: Proxy<Core>,
    ) -> std::io::Result<()> {
        let (event, footer) = connection.decode_message::<Events, CoreFooter>(header)?;
        footer.map(|f| connection.update_generation(&f));

        trace!("got event: {event:?}");

        match event {
            Events::Info(info) => {
                let props = Properties::new_vec(info.props.data);

                let core_info = CoreInfo {
                    id: info.id as Id,
                    cookie: info.cookie as u32,
                    user_name: info.user_name.as_str(),
                    host_name: info.host_name.as_str(),
                    version: info.version.as_str(),
                    name: info.name.as_str(),
                    mask: CoreChangeMask::from_bits_truncate(info.change_mask as u32),
                    props: Some(&props),
                };

                proxy_object_notify!(proxy, info, &core_info);
            }
            Events::Done(done) => {
                proxy_object_notify!(proxy, done, done.id as Id, done.seq as u32);
            }
            Events::Ping(ping) => {
                proxy_object_notify!(proxy, ping, ping.id as Id, ping.seq as u32);
            }
            Events::Error(err) => {
                proxy_object_notify!(
                    proxy,
                    error,
                    err.id as Id,
                    err.seq as u32,
                    err.res as u32,
                    &err.message
                );
            }
            Events::RemoveId(rem) => {
                proxy_object_notify!(proxy, remove_id, rem.id as Id);
            }
            Events::BoundId(bound) => {
                proxy_object_notify!(proxy, bound_id, bound.id as Id, bound.global_id as Id);
            }
            Events::AddMem(_) => {
                todo!("Core::AddMem is not yet implemented");
            }
            Events::RemoveMem(_) => {
                todo!("Core::RemoveMem is not yet implemented");
            }
            Events::BoundProps(bound) => {
                let props = Properties::new_vec(bound.props.data);

                proxy_object_notify!(
                    proxy,
                    bound_props,
                    bound.id as Id,
                    bound.global_id as Id,
                    &props
                );
            }
        }

        Ok(())
    }
}
