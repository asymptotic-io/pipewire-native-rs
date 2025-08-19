// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

#![warn(missing_docs)]
#![doc = include_str!("../../README.md")]

use std::sync::OnceLock;

use pipewire_native_spa as spa;

use properties::Properties;
use support::Support;

/// The [`context::Context`] holds local client configuration and support libraries. It is the entry point
/// to all other API.
pub mod context;
/// The [`core::Core`] object is the top-level singleton representing a connection to the PipeWire
/// server. The [`core::Core`] can be used to query objects known to the PipeWire server via the
/// [`proxy::registry::Registry`]. It can also be used to create objects on the server on behalf of this client
/// (this functionality has not yet been implemented).
pub mod core;
/// Contains a number of well-known keys used in properties (such as application name, language,
/// process ID, etc.).
pub mod keys;
/// Logggng utilities for using the PipeWire logging system. Controlled via the `PIPEWIRE_DEBUG` and
/// `PIPEWIRE_LOG*` environment variables, as well as [`context::Context`] properties.
pub mod log;
/// Provides an event loop for the library to communicate with the PipeWire server, as well as
/// primitives to managed mutually exclusive access to shared data structures.
pub mod main_loop;
/// Sructures for representing permissions.
pub mod permission;
/// Properties represent a key-value structure for various object properties. While the internal
/// representation is [`String`] pairs, helper methods are provided for interpreting values as
/// various primitives.
pub mod properties;
/// Proxy objects that represent objects exposed by the PipeWire server. This is the primary means
/// by which clients can interact with server-side objects.
pub mod proxy;
/// List of well-known types and interfaces shared by PipeWire server and clients.
pub mod types;

mod conf;
mod id_map;
mod protocol;
mod refcounted;
mod support;
mod utils;

#[doc(inline)]
pub use refcounted::*;

// pub use so users of closure! don't need to import paste themselves
#[doc(hidden)]
pub use paste::paste;

/// Represents an object identifier
pub type Id = u32;

#[allow(unused)]
pub(crate) const INVALID_ID: Id = Id::MAX;

pub(crate) static GLOBAL_SUPPORT: OnceLock<Support> = OnceLock::new();

/// Must be called before using any other API from this crate. Initialises global support libraries
/// and sets up logging.
pub fn init() {
    GLOBAL_SUPPORT.get_or_init(|| {
        let mut support = Support::new();

        let levels = log::parse_levels(std::env::var("PIPEWIRE_DEBUG").ok().as_deref());
        log::topic::init(&levels);

        // First, initialise logging
        let mut log_info = Properties::new();
        log_info.set(
            spa::interface::log::LEVEL,
            if support.no_color {
                "false".to_string()
            } else {
                utils::read_env_string("PIPEWIRE_LOG_COLOR", "true")
            },
        );
        log_info.set(
            spa::interface::log::TIMESTAMP,
            utils::read_env_string("PIPEWIRE_LOG_TIMESTAMP", "true"),
        );
        log_info.set(
            spa::interface::log::LINE,
            utils::read_env_string("PIPEWIRE_LOG_LINE", "true"),
        );
        let _ = std::env::var("PIPEWIRE_LOG").map(|v| {
            log_info.set(spa::interface::log::FILE, v);
        });

        // Initialise to the global default as parsed (if not specified, parse_levels() always
        // provides a default
        log_info.set(
            spa::interface::log::LEVEL,
            format!(
                "{}",
                levels.iter().find(|v| v.0.is_empty()).unwrap().1 as u32
            ),
        );

        // TODO: Check for/load the systemd logger if PIPEWIRE_SYSTEMD is set
        support
            .load_interfaces(
                spa::interface::plugin::LOG_FACTORY,
                &[spa::interface::LOG],
                Some(&log_info),
            )
            .expect("failed to load log interface");

        // Next, load CPU support
        let mut cpu_info = Properties::new();
        let _ = std::env::var("PIPEWIRE_CPU").map(|v| {
            cpu_info.set(spa::interface::cpu::FORCE, v);
        });
        let _ = std::env::var("PIPEWIRE_VM").map(|v| {
            cpu_info.set(spa::interface::cpu::VM, v);
        });

        support
            .load_interfaces(
                spa::interface::plugin::CPU_FACTORY,
                &[spa::interface::CPU],
                Some(&cpu_info),
            )
            .expect("failed to load CPU interface");

        support.init_log();

        // TODO: Load i18n interface

        support
            .load_interfaces(
                spa::interface::plugin::SYSTEM_FACTORY,
                &[spa::interface::SYSTEM],
                None,
            )
            .expect("failed to load system interface");
        support.init_system();

        support
    });
}

#[macro_export]
macro_rules! closure {
    ([$($names:ident <- $objects:ident),* $(^($($clones:ident),+))? $(^mut($($mut_clones:ident),+))?] $($($args:ident),* ,)? $body:block) => {
        $crate::paste! {
            {
                $(let [<_weak $names>] = $objects.downgrade();)*
                $($(let [<_cloned $clones>] = $clones.clone();)+)?
                $($(let mut [< _cloned $mut_clones >] = $mut_clones.clone();)+)?
                Box::new(move |$($($args),*)?| {
                    $(let $names = [<_weak $names>].upgrade().unwrap();)*
                    $($(let $clones = &[< _cloned $clones >];)+)?
                    $($(let $mut_clones = &mut [< _cloned $mut_clones >];)+)?
                    $body
                })
            }
        }
    };
    ([$($objects:ident),* $(^($($clones:ident),+))? $(^mut($($mut_clones:ident),+))?] $($($args:ident),* ,)? $body:block) => {
        $crate::closure!([$($objects <- $objects),* $(^($($clones),+))? $(^mut($($mut_clones),+))?] $($($args),*,)? $body)
    };
}

#[macro_export]
macro_rules! some_closure {
    ($($args:tt)*) => { Some($crate::closure!($($args)*)) };
}
