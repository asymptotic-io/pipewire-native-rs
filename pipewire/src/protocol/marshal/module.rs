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
        module::{Module, ModuleChangeMask, ModuleInfo},
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
    name: String,
    file_name: String,
    args: Option<String>,
    change_mask: i64,
    props: PairList<String, String>,
}

impl Events {
    pub(crate) fn demarshal(
        connection: &Connection,
        header: &super::message::Header,
        proxy: Proxy<Module>,
    ) -> std::io::Result<()> {
        let event = connection.decode_core_message::<Events>(header)?;

        trace!("got event: {event:?}");

        match event {
            Events::Info(info) => {
                let props = Properties::new_vec(info.props.data);

                let module_info = ModuleInfo {
                    id: info.id as Id,
                    name: info.name.as_str(),
                    file_name: info.file_name.as_str(),
                    args: info.args.as_deref(),
                    mask: ModuleChangeMask::from_bits_truncate(info.change_mask as u32),
                    props: &props,
                };

                proxy_object_notify!(proxy, info, &module_info);
            }
        }

        Ok(())
    }
}
