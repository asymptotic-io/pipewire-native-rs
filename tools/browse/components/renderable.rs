// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use tuirealm::props::TextSpan;

use pipewire::{
    keys,
    properties::Properties,
    proxy::{client::Client, device::Device, module::Module, HasProxy},
};

// Enough to render PW objects in a list view or detail view
pub trait Renderable {
    fn title(&self, props: &Properties) -> TextSpan;
}

impl Renderable for Client {
    fn title(&self, props: &Properties) -> TextSpan {
        TextSpan::from(format!(
            "#{}: {}",
            self.proxy().bound_id().unwrap_or(self.proxy().id()),
            props.get(keys::APP_NAME).unwrap_or("unknown"),
        ))
    }
}

impl Renderable for Device {
    fn title(&self, props: &Properties) -> TextSpan {
        TextSpan::from(format!(
            "#{}: {} ({})",
            self.proxy().bound_id().unwrap_or(self.proxy().id()),
            props.get("device.name").unwrap_or("unknown"),
            props.get("device.nick").unwrap_or("unknown"),
        ))
    }
}

impl Renderable for Module {
    fn title(&self, props: &Properties) -> TextSpan {
        TextSpan::from(format!(
            "#{}: {}",
            self.proxy().bound_id().unwrap_or(self.proxy().id()),
            props.get("module.name").unwrap_or("unknown"),
        ))
    }
}
