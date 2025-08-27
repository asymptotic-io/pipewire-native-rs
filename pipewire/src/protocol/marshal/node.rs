// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa::{
    self as spa,
    param::{ParamInfoFlags, ParamType},
    pod::{builder::Builder, types::Id as SpaId, Pod, RawPodOwned},
};

use crate::{
    closure, default_topic, log,
    properties::Properties,
    protocol::connection::Connection,
    proxy::{
        node::{Node, NodeChangeMask, NodeInfo, NodeMethods, NodeState},
        Proxy,
    },
    proxy_object_notify, trace, warn, Id,
};

use super::PairList;

default_topic!(log::topic::PROTOCOL);

#[repr(u8)]
#[derive(Debug, macros::Marshallable)]
pub(crate) enum Methods {
    SubscribeParams(SubscribeParams) = 1,
    EnumParams(EnumParams),
    SetParam(SetParam),
    SendCommand(SendCommand),
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct SubscribeParams {
    ids: Vec<SpaId<ParamType>>,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct EnumParams {
    seq: i32,
    id: SpaId<u32>,
    index: i32,
    num: i32,
    filter: spa::pod::RawPodOwned,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct SetParam {
    id: SpaId<ParamType>,
    flags: i32,
    param: spa::pod::RawPodOwned,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct SendCommand {
    command: spa::pod::RawPodOwned,
}

impl Methods {
    pub(crate) fn marshal(connection: Connection) -> NodeMethods<Node> {
        NodeMethods {
            subscribe_params: closure!([connection] proxy, ids, {
                connection.push(
                    proxy.id(),
                    Methods::SubscribeParams(SubscribeParams {
                        ids: ids.iter().map(|id| SpaId(*id)).collect::<Vec<_>>()
                    })
                )
            }),
            enum_params: closure!([connection] proxy, seq, id, index, num, filter, {
                let id = match id {
                    Some(id) => id as u32,
                    None => crate::ANY_ID,
                };

                let mut filter_data = [0u8; 16384];
                match filter {
                    Some(param_builder) => {
                        let builder = Builder::new(filter_data.as_mut_slice());
                        builder
                            .push_object(
                                param_builder.object_type,
                                param_builder.param_id,
                                param_builder.builder
                            )
                            .build()
                            .map_err(|e| {
                                std::io::Error::new(
                                    std::io::ErrorKind::InvalidData,
                                    format!("Could not build param: {e:?}")
                                )
                            })?;
                    },
                    None => {
                        ()
                            .encode(filter_data.as_mut_slice()).unwrap();
                    },
                };

                let filter = RawPodOwned::wrap(Vec::from(filter_data)).unwrap();

                connection.push(
                    proxy.id(),
                    Methods::EnumParams(EnumParams {
                        seq: seq as i32,
                        id: SpaId(id),
                        index: index as i32,
                        num: num as i32,
                        filter,
                    })
                )
            }),
            set_param: closure!([connection] proxy, param_id, object_type, flags, param_builder, {
                let mut param_data = [0u8; 16384];

                let builder = Builder::new(param_data.as_mut_slice());
                builder
                    .push_object(
                        object_type,
                        param_id,
                        param_builder
                    )
                    .build()
                    .map_err(|e| {
                        std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            format!("Could not build param: {e:?}")
                        )
                    })?;

                let param = RawPodOwned::wrap(Vec::from(param_data)).unwrap();

                connection.push(
                    proxy.id(),
                    Methods::SetParam(SetParam {
                        id: SpaId(param_id),
                        flags: flags as i32,
                        param
                    })
                )
            }),
            send_command: closure!([connection] proxy, command_builder, {
                let mut command_data = [0u8; 16384];

                let builder = Builder::new(command_data.as_mut_slice());

                let command_built = command_builder(builder).build().map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("Could not build command: {e:?}")
                    )
                })?;

                let command = RawPodOwned::wrap(Vec::from(command_built)).unwrap();

                connection.push(
                    proxy.id(),
                    Methods::SendCommand(SendCommand { command })
                )
            }),
        }
    }
}

#[derive(Debug, macros::Marshallable)]
pub(crate) enum Events {
    Info(Info),
    Param(Param),
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Info {
    id: i32,
    max_input_ports: i32,
    max_output_ports: i32,
    change_mask: i64,
    n_input_ports: i32,
    n_output_ports: i32,
    state: SpaId<NodeState>,
    error: Option<String>,
    props: PairList<String, String>,
    param_info: PairList<SpaId<ParamType>, i32>,
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Param {
    seq: i32,
    id: SpaId<ParamType>,
    index: i32,
    next: i32,
    param: RawPodOwned,
}

impl Events {
    pub(crate) fn demarshal(
        connection: &Connection,
        header: &super::message::Header,
        proxy: Proxy<Node>,
    ) -> std::io::Result<()> {
        let event = connection.decode_core_message::<Events>(header)?;

        trace!("got event: {event:?}");

        match event {
            Events::Info(info) => {
                let props = Properties::new_vec(info.props.data);
                let mut param_info = vec![];

                for (id, flags) in info.param_info.data {
                    match ParamInfoFlags::from_bits(flags as u32) {
                        Some(flags) => param_info.push((id.0, flags)),
                        None => {
                            warn!("Invalid param info: {id:?} {flags}");
                        }
                    }
                }

                let node_info = NodeInfo {
                    id: info.id as Id,
                    max_input_ports: info.max_input_ports as u32,
                    max_output_ports: info.max_output_ports as u32,
                    mask: NodeChangeMask::from_bits_truncate(info.change_mask as u32),
                    n_input_ports: info.n_input_ports as u32,
                    n_output_ports: info.n_output_ports as u32,
                    state: info.state.0,
                    error: info.error.as_deref(),
                    props: &props,
                    params: param_info.as_slice(),
                };

                proxy_object_notify!(proxy, info, &node_info);
            }
            Events::Param(param) => {
                let seq = param.seq as u32;
                let param_id = param.id.0;
                let index = param.index as u32;
                let next = param.next as u32;
                let param_pod = param.param;
                proxy_object_notify!(proxy, param, seq, param_id, index, next, &param_pod);
            }
        }

        Ok(())
    }
}
