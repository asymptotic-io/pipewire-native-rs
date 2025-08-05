// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa::{self as spa, pod::Pod};

use crate::{
    closure,
    core::{Core, CoreChangeMask, CoreInfo, CoreMethods},
    protocol::{connection::Connection, ASYNC_SEQ_BIT, ASYNC_SEQ_MASK},
    proxy::Proxy,
    proxy_object_notify, Id,
};

use super::{Marshallable, PairList};

#[derive(Debug)]
pub(crate) enum Methods {
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

impl Methods {
    pub(crate) fn marshal(connection: Connection) -> CoreMethods<Core> {
        CoreMethods {
            hello: closure!(connection, proxy, version, {
                connection.push(
                    proxy.id(),
                    Methods::Hello(Hello {
                        version: version as i32,
                    }),
                )
            }),
            sync: closure!(connection, proxy, id, {
                let seq = ASYNC_SEQ_BIT | (connection.next_seq() & ASYNC_SEQ_MASK);
                connection.push(
                    proxy.id(),
                    Methods::Sync(Sync {
                        id: id as i32,
                        seq: seq as i32,
                    }),
                )
            }),
            pong: closure!(connection, proxy, id, seq, {
                connection.push(
                    proxy.id(),
                    Methods::Pong(Pong {
                        id: id as i32,
                        seq: seq as i32,
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
}

#[derive(Debug)]
pub(crate) enum Events {
    Info(Info),
    Done(Done),
    Ping(Ping),
    Error(Error),
    RemoveId(RemoveId),
    BoundId(BoundId),
    AddMem(AddMem),
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
pub(crate) struct Error {
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
pub(crate) struct BoundProps {
    id: i32,
    global_id: i32,
    props: PairList<String, String>,
}

impl Marshallable for Events {
    fn opcode(&self) -> u8 {
        match self {
            Self::Info(_) => 0,
            Self::Done(_) => 1,
            Self::Ping(_) => 2,
            Self::Error(_) => 3,
            Self::RemoveId(_) => 4,
            Self::BoundId(_) => 5,
            Self::AddMem(_) => 6,
            Self::BoundProps(_) => 7,
        }
    }

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error> {
        match self {
            Self::Info(o) => o.encode(data),
            Self::Done(o) => o.encode(data),
            Self::Ping(o) => o.encode(data),
            Self::Error(o) => o.encode(data),
            Self::RemoveId(o) => o.encode(data),
            Self::BoundId(o) => o.encode(data),
            Self::AddMem(o) => o.encode(data),
            Self::BoundProps(o) => o.encode(data),
        }
    }

    fn decode(opcode: u8, data: &[u8]) -> Result<(Self, usize), spa::pod::Error>
    where
        Self: Sized,
    {
        match opcode {
            0 => Info::decode(data).map(|(o, s)| (Self::Info(o), s)),
            1 => Done::decode(data).map(|(o, s)| (Self::Done(o), s)),
            2 => Ping::decode(data).map(|(o, s)| (Self::Ping(o), s)),
            3 => Error::decode(data).map(|(o, s)| (Self::Error(o), s)),
            4 => RemoveId::decode(data).map(|(o, s)| (Self::RemoveId(o), s)),
            5 => BoundId::decode(data).map(|(o, s)| (Self::BoundId(o), s)),
            6 => AddMem::decode(data).map(|(o, s)| (Self::AddMem(o), s)),
            7 => BoundProps::decode(data).map(|(o, s)| (Self::BoundProps(o), s)),
            _ => unreachable!(),
        }
    }
}

impl Events {
    pub(crate) fn demarshal(
        connection: &Connection,
        header: &super::Header,
        proxy: Proxy<Core>,
    ) -> std::io::Result<()> {
        let event = connection.decode_message::<Events>(header)?;

        match event {
            Events::Info(info) => {
                let props = spa::dict::Dict::new(info.props.data);

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
            Events::BoundProps(bound) => {
                let props = spa::dict::Dict::new(bound.props.data);
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
