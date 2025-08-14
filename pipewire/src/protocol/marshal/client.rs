// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa::{self as spa, pod::Pod};

use crate::{
    closure, default_topic, log,
    permission::{self, PermissionBits},
    properties::Properties,
    protocol::connection::Connection,
    proxy::{
        client::{Client, ClientChangeMask, ClientInfo, ClientMethods},
        Proxy,
    },
    proxy_object_notify, trace, Id,
};

use super::PairList;

default_topic!(log::topic::PROTOCOL);

#[repr(u8)]
#[derive(Debug, macros::Marshallable)]
pub(crate) enum Methods {
    Error(Error) = 1,
    UpdateProperties(UpdateProperties),
    GetPermissions(GetPermissions),
    UpdatePermissions(UpdatePermissions),
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Error {
    id: i32,
    res: i32,
    message: String,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct UpdateProperties {
    props: PairList<String, String>,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct GetPermissions {
    index: i32,
    num: i32,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct UpdatePermissions {
    permissions: PairList<i32, i32>,
}

impl Methods {
    pub(crate) fn marshal(connection: Connection) -> ClientMethods<Client> {
        ClientMethods {
            error: closure!([connection] proxy, id, res, message, {
                connection.push(
                    proxy.id(),
                    Methods::Error(Error {
                        id: id as i32,
                        res: res as i32,
                        message: message.to_string(),
                    }),
                )
            }),
            update_properties: closure!([connection] proxy, props, {
                connection.push(
                    proxy.id(),
                    Methods::UpdateProperties(UpdateProperties {
                        props: PairList::new(
                            props
                                .iter()
                                .map(|(k, v)| (k.to_string(), v.to_string()))
                                .collect(),
                        ),
                    }),
                )
            }),
            get_permissions: closure!([connection] proxy, idx, num, {
                connection.push(
                    proxy.id(),
                    Methods::GetPermissions(GetPermissions {
                        index: idx as i32,
                        num: num as i32,
                    }),
                )
            }),
            update_permissions: closure!([connection] proxy, permissions, {
                let permissions = PairList::new(
                    permissions
                        .iter()
                        .map(|p| (p.id as i32, p.permissions.bits() as i32))
                        .collect(),
                );

                connection.push(
                    proxy.id(),
                    Methods::UpdatePermissions(UpdatePermissions { permissions }),
                )
            }),
        }
    }
}

#[derive(Debug, macros::Marshallable)]
pub(crate) enum Events {
    Info(Info),
    Permissions(Permissions),
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Info {
    id: i32,
    change_mask: i64,
    props: PairList<String, String>,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Permissions {
    index: i32,
    permissions: PairList<i32, i32>,
}

impl Events {
    pub(crate) fn demarshal(
        connection: &Connection,
        header: &super::message::Header,
        proxy: Proxy<Client>,
    ) -> std::io::Result<()> {
        let event = connection.decode_message::<Events>(header)?;

        trace!("got event: {event:?}");

        match event {
            Events::Info(info) => {
                let props = Properties::new_vec(info.props.data);

                let client_info = ClientInfo {
                    id: info.id as Id,
                    mask: ClientChangeMask::from_bits_truncate(info.change_mask as u32),
                    props: &props,
                };

                proxy_object_notify!(proxy, info, &client_info);
            }
            Events::Permissions(permissions) => {
                let perms: Vec<permission::Permission> = permissions
                    .permissions
                    .data
                    .iter()
                    .map(|(id, p)| permission::Permission {
                        id: *id as Id,
                        permissions: PermissionBits::from_bits_truncate(*p as u32),
                    })
                    .collect();
                proxy_object_notify!(proxy, permissions, permissions.index as u32, &perms)
            }
        }

        Ok(())
    }
}
