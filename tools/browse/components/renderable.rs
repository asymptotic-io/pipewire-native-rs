// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use tuirealm::props::TextSpan;

use pipewire::{keys, properties::Properties, proxy::HasProxy};

use crate::pw::{
    ClientDetails, DeviceDetails, FactoryDetails, LinkDetails, MetadataDetails, ModuleDetails,
    NodeDetails, PortDetails,
};

// Enough to render PW objects in a list view or detail view
pub trait Renderable {
    fn props(&self) -> &Properties;
    fn params(&self) -> Vec<(&spa::param::ParamType, &crate::pw::Params)> {
        vec![]
    }
    fn title(&self) -> TextSpan;
}

impl Renderable for ClientDetails {
    fn props(&self) -> &Properties {
        &self.props
    }

    fn title(&self) -> TextSpan {
        TextSpan::from(format!(
            "#{}: {}",
            self.client
                .proxy()
                .bound_id()
                .unwrap_or(self.client.proxy().id()),
            self.props.get(keys::APP_NAME).unwrap_or("unknown"),
        ))
    }
}

impl Renderable for DeviceDetails {
    fn props(&self) -> &Properties {
        &self.props
    }

    fn params(&self) -> Vec<(&spa::param::ParamType, &crate::pw::Params)> {
        self.params.iter().collect()
    }

    fn title(&self) -> TextSpan {
        TextSpan::from(format!(
            "#{}: {} ({})",
            self.device
                .proxy()
                .bound_id()
                .unwrap_or(self.device.proxy().id()),
            self.props.get("device.name").unwrap_or("unknown"),
            self.props.get("device.nick").unwrap_or("unknown"),
        ))
    }
}

impl Renderable for FactoryDetails {
    fn props(&self) -> &Properties {
        &self.props
    }

    fn title(&self) -> TextSpan {
        TextSpan::from(format!(
            "#{}: {}",
            self.factory
                .proxy()
                .bound_id()
                .unwrap_or(self.factory.proxy().id()),
            self.props.get("factory.name").unwrap_or("unknown"),
        ))
    }
}

impl Renderable for LinkDetails {
    fn props(&self) -> &Properties {
        &self.props
    }

    fn title(&self) -> TextSpan {
        TextSpan::from(format!(
            "#{}: {}/{} -> {}/{}",
            self.link
                .proxy()
                .bound_id()
                .unwrap_or(self.link.proxy().id()),
            self.props.get("link.output.node").unwrap_or("unknown"),
            self.props.get("link.output.port").unwrap_or("unknown"),
            self.props.get("link.input.node").unwrap_or("unknown"),
            self.props.get("link.input.port").unwrap_or("unknown"),
        ))
    }
}

impl Renderable for MetadataDetails {
    fn props(&self) -> &Properties {
        &self.props
    }

    fn title(&self) -> TextSpan {
        TextSpan::from(format!(
            "#{}: {}",
            self.metadata
                .proxy()
                .bound_id()
                .unwrap_or(self.metadata.proxy().id()),
            self.props.get("metadata.name").unwrap_or("unknown"),
        ))
    }
}

impl Renderable for ModuleDetails {
    fn props(&self) -> &Properties {
        &self.props
    }

    fn title(&self) -> TextSpan {
        TextSpan::from(format!(
            "#{}: {}",
            self.module
                .proxy()
                .bound_id()
                .unwrap_or(self.module.proxy().id()),
            self.props.get("module.name").unwrap_or("unknown"),
        ))
    }
}

impl Renderable for NodeDetails {
    fn props(&self) -> &Properties {
        &self.props
    }

    fn params(&self) -> Vec<(&spa::param::ParamType, &crate::pw::Params)> {
        self.params.iter().collect()
    }

    fn title(&self) -> TextSpan {
        TextSpan::from(format!(
            "#{}: {} ({})",
            self.node
                .proxy()
                .bound_id()
                .unwrap_or(self.node.proxy().id()),
            self.props.get("node.name").unwrap_or("unknown"),
            self.props.get("node.description").unwrap_or("unknown"),
        ))
    }
}

impl Renderable for PortDetails {
    fn props(&self) -> &Properties {
        &self.props
    }

    fn params(&self) -> Vec<(&spa::param::ParamType, &crate::pw::Params)> {
        self.params.iter().collect()
    }

    fn title(&self) -> TextSpan {
        TextSpan::from(format!(
            "#{}: {}",
            self.port
                .proxy()
                .bound_id()
                .unwrap_or(self.port.proxy().id()),
            self.props.get("port.alias").unwrap_or("unknown"),
        ))
    }
}
