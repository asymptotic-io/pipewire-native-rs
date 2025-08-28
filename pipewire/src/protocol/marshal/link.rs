// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa::{self as spa, pod::Pod};

use crate::{
    default_topic, log,
    properties::Properties,
    protocol::connection::Connection,
    proxy::{
        link::{Link, LinkChangeMask, LinkInfo, LinkState},
        Proxy,
    },
    proxy_object_notify, trace, Id,
};

use super::PairList;

default_topic!(log::topic::PROTOCOL);

// No methods

#[derive(Debug, macros::Marshallable)]
pub(crate) enum Events {
    Info(Info),
}

#[derive(Debug, macros::PodStruct)]
pub(crate) struct Info {
    id: i32,
    output_node_id: i32,
    output_port_id: i32,
    input_node_id: i32,
    input_port_id: i32,
    change_mask: i64,
    state: i32,
    error: Option<String>,
    format: spa::pod::RawPodOwned,
    props: PairList<String, String>,
}

impl Events {
    pub(crate) fn demarshal(
        connection: &Connection,
        header: &super::message::Header,
        proxy: Proxy<Link>,
    ) -> std::io::Result<()> {
        let event = connection.decode_core_message::<Events>(header)?;

        trace!("got event: {event:?}");

        match event {
            Events::Info(info) => {
                let props = Properties::new_vec(info.props.data);
                let format = match info.format.decode::<()>() {
                    Ok(()) => Some(&info.format),
                    Err(_) => None,
                };

                let link_info = LinkInfo {
                    id: info.id as Id,
                    output_node_id: info.output_node_id as Id,
                    output_port_id: info.output_port_id as Id,
                    input_node_id: info.input_node_id as Id,
                    input_port_id: info.input_port_id as Id,
                    mask: LinkChangeMask::from_bits_truncate(info.change_mask as u32),
                    state: LinkState::try_from(info.state as u32).unwrap(),
                    error: info.error.as_deref(),
                    format,
                    props: &props,
                };

                proxy_object_notify!(proxy, info, &link_info);
            }
        }

        Ok(())
    }
}
