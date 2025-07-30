// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::sync::OnceLock;

use pipewire_native_spa as spa;

use properties::Properties;
use support::Support;

pub mod conf;
pub mod context;
pub mod core;
mod id_map;
pub mod keys;
pub mod log;
pub mod main_loop;
pub mod permission;
pub mod properties;
mod protocol;
pub mod proxy;
pub mod types;

mod support;
mod utils;

pub type Id = u32;
pub const INVALID_ID: Id = Id::MAX;

pub(crate) static GLOBAL_SUPPORT: OnceLock<Support> = OnceLock::new();

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

pub trait Refcounted: Clone {
    type WeakRef;

    fn upgrade(this: &Self::WeakRef) -> Option<Self>
    where
        Self: Sized;
    fn downgrade(&self) -> Self::WeakRef;
}

#[macro_export]
macro_rules! refcounted {
    (
        // FIXME: bounds can be non-types, so we probably need something that munches tts
        $(#[$(attrs:meta)+])?
        $visibility:vis struct $name:ident $(<$($generic:ident $(: $bound:ty)?),*>)? {
            $($body:tt)*
        }
    ) => {
        paste::paste! {
            #[derive(Clone)]
            $visibility struct $name $(<$($generic $(: $bound)?),*>)? {
                inner: Rc<[<Inner $name>] $(<$($generic),*>)?>,
            }

            #[derive(Clone)]
            pub struct [<Weak $name>] $(<$($generic $(: $bound)?),*>)? {
                inner: Weak<[<Inner $name>] $(<$($generic>)?),*>,
            }

            impl $(<$($generic $(: $bound)?),*>)? $name $(<$($generic),*>)? {
                pub fn downgrade(&self) -> [<Weak $name>] $(<$($generic),*>)? {
                    [<Weak $name>] {
                        inner: Rc::downgrade(&self.inner),
                    }
                }
            }

            impl $(<$($generic $(: $bound)?),*>)? [<Weak $name>] $(<$($generic),*>)? {
                pub fn upgrade(&self) -> Option<$name $(<$($generic),*>)?> {
                    self.inner.upgrade().map(|inner| $name { inner })
                }
            }

            impl $(<$($generic $(: $bound)?),*>)? crate::Refcounted for $name $(<$($generic),*>)? {
                type WeakRef = [<Weak $name>] $(<$($generic),*>)?;

                fn upgrade(this: &Self::WeakRef) -> Option<Self> {
                    this.upgrade()
                }

                fn downgrade(&self) -> Self::WeakRef {
                    self.downgrade()
                }
            }

            struct [<Inner $name>] $(<$($generic $(: $bound)?),*>)? {
                $($body)*
            }
        }
    }
}
