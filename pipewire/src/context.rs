// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    cell::RefCell,
    ffi::CStr,
    pin::Pin,
    rc::Rc,
    sync::{Arc, LazyLock},
};

use crate::{
    conf, core::Core, debug, default_topic, keys, log, main_loop::MainLoop, properties::Properties,
    protocol::Protocol, refcounted, GLOBAL_SUPPORT,
};

use pipewire_native_spa::{
    self as spa,
    interface::{
        r#loop::{LoopImpl, LoopUtilsImpl},
        system::SystemImpl,
    },
};

default_topic!(log::topic::CONTEXT);

refcounted! {
    pub struct Context {
        main_loop: MainLoop,
        properties: RefCell<Properties>,
        conf: Properties,
        // In the C implementation, this is a list, but in practice there is only native-protocol
        protocol: Protocol,
        #[allow(dead_code)]
        system: Arc<Pin<Box<SystemImpl>>>,
        #[allow(dead_code)]
        loop_: Arc<Pin<Box<LoopImpl>>>,
        #[allow(dead_code)]
        loop_utils: Arc<Pin<Box<LoopUtilsImpl>>>,
    }
}

static PROCESS_NAME: LazyLock<String> = LazyLock::new(|| {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(name_part) = exe.file_name() {
            if let Some(name_str) = name_part.to_str() {
                return name_str.to_string();
            }
        }
    }

    // Fallback to the process ID if we can't get the name
    format!("pid-{}", std::process::id())
});

impl Context {
    pub fn new(main_loop: &MainLoop, properties: Properties) -> std::io::Result<Self> {
        let inner = InnerContext::new(main_loop.clone(), properties)?;
        let context = Context {
            inner: Rc::new(inner),
        };

        // Provide (weak) reference to inner descendants that need it
        context.inner.protocol.set_context(context.downgrade());

        Ok(context)
    }

    pub fn main_loop(&self) -> MainLoop {
        self.inner.main_loop.clone()
    }

    pub fn properties(&self) -> spa::dict::Dict {
        self.inner.properties.borrow().dict()
    }

    pub(crate) fn update_properties(&self, props: &Properties, keys: Vec<&str>) {
        self.inner
            .properties
            .borrow_mut()
            .update_keys(props.iter_str(), keys);
    }

    pub(crate) fn protocol(&self) -> &Protocol {
        &self.inner.protocol
    }

    pub fn connect(&self, properties: Option<Properties>) -> std::io::Result<Core> {
        Core::new(self, properties.unwrap_or(Properties::new()))
    }
}

impl InnerContext {
    fn new(main_loop: MainLoop, properties: Properties) -> std::io::Result<Self> {
        // TODO: plugin loader interface
        let support = main_loop.support();
        let system = GLOBAL_SUPPORT
            .get()
            .expect("Global support should be initialised")
            .system()
            .clone();
        let loop_ = support.loop_.clone();
        let loop_utils = support.loop_utils.clone();

        let mut this = InnerContext {
            main_loop,
            properties: RefCell::new(properties),
            conf: Properties::new(),
            protocol: Protocol::new("protocol-native"),
            system,
            loop_,
            loop_utils,
        };

        debug!("Creating context");

        this.set_default_properties();
        this.load_conf()?;

        let cpu = super::GLOBAL_SUPPORT.get().map(|v| v.cpu());
        let vm_type = match &cpu {
            Some(cpu) => cpu.get_vm_type(),
            None => spa::interface::cpu::CpuVm::None,
        };

        if vm_type != spa::interface::cpu::CpuVm::None {
            this.properties
                .borrow_mut()
                .set("cpu.vm.name", vm_type.to_string());
        }

        // TODO: add overrides and rules from context.properties

        if let Ok(core_name) = std::env::var("PIPEWIRE_CORE") {
            this.properties.borrow_mut().set(keys::CORE_NAME, core_name);
        }

        if let Some(cpu) = &cpu {
            if this.properties.borrow().get(keys::CPU_MAX_ALIGN).is_none() {
                this.properties
                    .borrow_mut()
                    .set(keys::CPU_MAX_ALIGN, cpu.get_max_align().to_string());
            }
        }

        if this
            .properties
            .borrow()
            .get_bool("mem.mlock-all")
            .unwrap_or(false)
        {
            unsafe {
                libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE);
            }
        }

        // TODO: create/use default settings
        // TODO: start data loops

        // TODO: create a mempool
        // TODO: create a work queue

        // TODO: D-Bus...

        // TODO: Create impl core and register

        // TODO: Start data loops

        // TODO: Expose settings

        Ok(this)
    }

    fn load_conf(&mut self) -> std::io::Result<()> {
        let conf_prefix = std::env::var("PIPEWIRE_CONFIG_PREFIX").ok().or_else(|| {
            self.properties
                .borrow()
                .get(keys::CONFIG_PREFIX)
                .map(String::from)
        });

        let conf_name = std::env::var("PIPEWIRE_CONFIG_NAME")
            .ok()
            .or_else(|| {
                self.properties
                    .borrow()
                    .get(keys::CONFIG_NAME)
                    .map(String::from)
            })
            .and_then(|s| if s == "client-rt.conf" { None } else { Some(s) })
            .unwrap_or("client.conf".to_string());

        conf::load(conf_prefix.as_deref(), &conf_name, &mut self.conf)?;

        // TODO: overrides

        Ok(())
    }

    fn set_default_properties(&mut self) {
        if self.properties.borrow().get(keys::APP_NAME).is_none() {
            self.properties
                .borrow_mut()
                .set(keys::APP_NAME, PROCESS_NAME.clone());
        };
        if self
            .properties
            .borrow()
            .get(keys::APP_PROCESS_BINARY)
            .is_none()
        {
            self.properties
                .borrow_mut()
                .set(keys::APP_NAME, PROCESS_NAME.clone());
        };
        if self.properties.borrow().get(keys::APP_LANGUAGE).is_none() {
            if let Ok(lang) = std::env::var("LANG") {
                self.properties.borrow_mut().set(keys::APP_LANGUAGE, lang);
            }
        };
        if self.properties.borrow().get(keys::APP_PROCESS_ID).is_none() {
            self.properties
                .borrow_mut()
                .set(keys::APP_PROCESS_ID, std::process::id().to_string());
        };
        if self
            .properties
            .borrow()
            .get(keys::APP_PROCESS_USER)
            .is_none()
        {
            if let Some(user) = unsafe {
                libc::getpwuid(libc::getuid())
                    .as_ref()
                    .map(|p| CStr::from_ptr(p.pw_name).to_string_lossy().to_string())
            } {
                self.properties
                    .borrow_mut()
                    .set(keys::APP_PROCESS_USER, user);
            }
        };
        if self
            .properties
            .borrow()
            .get(keys::APP_PROCESS_HOST)
            .is_none()
        {
            let mut name: [u8; 256] = [0; 256];
            unsafe { libc::gethostname(name.as_mut_ptr() as *mut i8, name.len() as libc::size_t) };
            if let Ok(hostname) = CStr::from_bytes_until_nul(&name) {
                self.properties.borrow_mut().set(
                    keys::APP_PROCESS_HOST,
                    hostname.to_string_lossy().to_string(),
                );
            }
        };
        if self
            .properties
            .borrow()
            .get(keys::APP_PROCESS_SESSION_ID)
            .is_none()
        {
            if let Ok(session_id) = std::env::var("XDG_SESSION_ID") {
                self.properties
                    .borrow_mut()
                    .set(keys::APP_PROCESS_SESSION_ID, session_id);
            }
        };
        if self
            .properties
            .borrow()
            .get(keys::WINDOW_X11_DISPLAY)
            .is_none()
        {
            if let Ok(display) = std::env::var("DISPLAY") {
                self.properties
                    .borrow_mut()
                    .set(keys::WINDOW_X11_DISPLAY, display);
            }
        };
    }
}
