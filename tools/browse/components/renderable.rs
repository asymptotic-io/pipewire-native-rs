// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use tuirealm::props::TextSpan;

use pipewire::{keys, properties::Properties, proxy::HasProxy};

use crate::pw::{ClientDetails, DeviceDetails, ModuleDetails};

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
