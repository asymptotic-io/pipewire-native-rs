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
    proxy_object_notify,
};

use super::{Marshallable, PairList};

#[derive(Debug)]
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

#[derive(Debug, macros::PodStruct)]
struct Hello {
    version: i32,
}

#[derive(Debug, macros::PodStruct)]
struct Sync {
    id: i32,
    seq: i32,
}

#[derive(Debug, macros::PodStruct)]
struct Pong {
    id: i32,
    seq: i32,
}

pub(crate) fn marshal_methods(connection: Connection) -> CoreMethods<Core> {
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
struct Info {
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
struct Done {
    id: i32,
    seq: i32,
}

#[derive(Debug, macros::PodStruct)]
struct Ping {
    id: i32,
    seq: i32,
}

#[derive(Debug, macros::PodStruct)]
struct Error {
    id: i32,
    seq: i32,
    res: i32,
    message: String,
}

#[derive(Debug, macros::PodStruct)]
struct RemoveId {
    id: i32,
}

#[derive(Debug, macros::PodStruct)]
struct BoundId {
    id: i32,
    global_id: i32,
}

#[derive(Debug, macros::PodStruct)]
struct AddMem {
    // TODO
}

#[derive(Debug, macros::PodStruct)]
struct BoundProps {
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
            _ => todo!(),
        }
    }

    fn encode(&self, data: &mut [u8]) -> Result<usize, pipewire_native_spa::pod::Error> {
        match self {
            Self::Info(o) => o.encode(data),
            _ => todo!(),
        }
    }

    fn decode(opcode: u8, data: &[u8]) -> Result<(Self, usize), spa::pod::Error>
    where
        Self: Sized,
    {
        match opcode {
            0 => Info::decode(data).map(|(o, s)| (Self::Info(o), s)),
            _ => todo!(),
        }
    }
}

pub(crate) fn demarshal_event(
    connection: &Connection,
    header: &super::Header,
    proxy: Proxy<Core>,
) -> std::io::Result<()> {
    let event = connection.decode_message::<Events>(header)?;

    match event {
        Events::Info(info) => {
            let mut core_info = CoreInfo {
                id: info.id as u32,
                cookie: info.cookie as u32,
                user_name: info.user_name.as_str(),
                host_name: info.host_name.as_str(),
                version: info.version.as_str(),
                name: info.name.as_str(),
                mask: CoreChangeMask::from_bits_truncate(info.change_mask as u32),
                props: None, /* set the reference in a way we don't lose it */
            };
            let props = spa::dict::Dict::new(info.props.data);
            core_info.props = Some(&props);

            proxy_object_notify!(proxy, info, &core_info);
        }
        _ => {}
    }

    Ok(())
}
