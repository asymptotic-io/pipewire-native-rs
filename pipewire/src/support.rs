// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use pipewire_native_spa as spa;

use crate::properties::Properties;
use crate::utils;

pub(crate) struct Support {
    // TODO: Implement when we have unload_spa_handle()
    _do_dlclose: bool,
    pub no_color: bool,
    pub no_config: bool,

    plugin_dirs: Vec<String>,
    support_lib: String,

    inner: Mutex<Inner>,
    log: Option<Arc<Pin<Box<spa::interface::log::LogImpl>>>>,
}

struct Inner {
    plugins: HashMap<String, spa::support::ffi::plugin::Plugin>,
    factories: HashMap<String, Box<dyn spa::interface::plugin::HandleFactory>>,
    handles: Vec<(String, Box<dyn spa::interface::plugin::Handle>)>,
    support: spa::interface::Support,
}

const SUPPORTLIB: &str = "support/libspa-support";

impl Support {
    pub fn new() -> Support {
        let do_dlclose = utils::read_env_bool("PIPEWIRE_DLCLOSE", false);
        let no_color = utils::read_env_bool("NO_COLOR", false);
        let no_config = utils::read_env_bool("PIPEWIRE_NO_CONFIG", false);
        /* FIXME: unhardcode */
        let plugin_dir = utils::read_env_string("SPA_PLUGIN_DIR", "/usr/lib64/spa-0.2")
            .split(':')
            .map(|s| s.to_string())
            .collect();
        let support_lib = std::env::var("SPA_SUPPORT_LIB").unwrap_or(SUPPORTLIB.to_string());

        Support {
            _do_dlclose: do_dlclose,
            no_config,
            no_color,
            plugin_dirs: plugin_dir,
            support_lib,
            inner: Mutex::new(Inner {
                plugins: HashMap::new(),
                factories: HashMap::new(),
                handles: Vec::new(),
                support: spa::interface::Support::new(),
            }),
            log: None,
        }
    }

    pub(super) fn init_log(&mut self) {
        let inner = self.inner.lock().unwrap();

        self.log = inner
            .support
            .get_interface::<spa::interface::log::LogImpl>(spa::interface::LOG);
    }

    pub fn log(&self) -> &Arc<Pin<Box<spa::interface::log::LogImpl>>> {
        self.log
            .as_ref()
            .expect("Log interface should be initialized")
    }

    pub fn cpu(&self) -> Arc<Pin<Box<spa::interface::cpu::CpuImpl>>> {
        let inner = self.inner.lock().unwrap();

        inner
            .support
            .get_interface::<spa::interface::cpu::CpuImpl>(spa::interface::CPU)
            .unwrap()
    }

    pub fn load_spa_handle(
        &mut self,
        lib: Option<&str>,
        factory_name: &str,
        info: Option<&Properties>,
    ) -> std::io::Result<Box<dyn spa::interface::plugin::Handle>> {
        let mut inner = self.inner.lock().unwrap();
        let lib = lib.unwrap_or(&self.support_lib);

        let mut lib_name = "".to_string();
        let mut plugin = None;

        for dir in self.plugin_dirs.iter() {
            let mut path = PathBuf::from(dir);
            path.push(format! {"{}.so", lib});

            lib_name = path.to_string_lossy().to_string();

            match inner.plugins.get(&lib_name) {
                Some(p) => {
                    plugin = Some(p);
                    break;
                }
                None => match spa::support::ffi::plugin::load(&path) {
                    Ok(p) => {
                        inner.plugins.insert(lib_name.to_string(), p);
                        plugin = inner.plugins.get(&lib_name);
                        break;
                    }
                    Err(_) => {
                        // Try the next directory
                        continue;
                    }
                },
            };
        }

        let plugin = plugin.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Plugin not found: {}", lib),
            )
        })?;

        let factory_key = format!("{}/{}", lib_name, factory_name);
        let factory = match inner.factories.get(&factory_key) {
            Some(factory) => factory,
            None => match plugin.find_factory(factory_name) {
                Some(factory) => {
                    inner.factories.insert(factory_key.clone(), factory);
                    inner.factories.get(&factory_key).unwrap()
                }
                None => {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("Factory not found: {}", factory_name),
                    ))
                }
            },
        };

        let handle = factory
            .init(info.map(|p| p.dict()), &inner.support)
            .map_err(|_| {
                std::io::Error::other(format!("Failed to initialize factory: {}", factory_name))
            })?;

        Ok(handle)
    }

    pub fn load_interface(
        &mut self,
        factory_name: &str,
        iface_type: &'static str,
        info: Option<&Properties>,
    ) -> std::io::Result<()> {
        let factory = self.load_spa_handle(None, factory_name, info)?;

        let iface = factory.get_interface(iface_type).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Interface not found: {}", iface_type),
            )
        })?;

        let mut inner = self.inner.lock().unwrap();

        inner.support.add_interface(iface_type, iface);
        inner.handles.push((factory_name.to_string(), factory));

        Ok(())
    }
}

unsafe impl Send for Support {}
unsafe impl Sync for Support {}
