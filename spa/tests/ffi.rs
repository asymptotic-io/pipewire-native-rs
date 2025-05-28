// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::path::PathBuf;

use pipewire_native_spa::dict::Dict;
use pipewire_native_spa::interface;
use pipewire_native_spa::interface::cpu::CpuImpl;
use pipewire_native_spa::interface::log::{LogImpl, LogLevel};
use pipewire_native_spa::interface::r#loop::{ControlMethodsImpl, LoopImpl, LoopUtilsImpl};
use pipewire_native_spa::interface::system::SystemImpl;
use pipewire_native_spa::support::ffi;

#[test]
fn test_load_support() {
    let mut support = interface::Support::new();

    let plugin_path = std::env::var("SPA_TEST_PLUGIN_PATH")
        .unwrap_or("/usr/lib64/spa-0.2/support/libspa-support.so".to_string());

    let plugin =
        ffi::plugin::load(&PathBuf::from(plugin_path)).expect("Plugin loading should not fail");

    let log_factory = plugin
        .find_factory(interface::plugin::LOG_FACTORY)
        .expect("Should find log factory");

    assert!(log_factory.info().is_none());

    let interfaces = log_factory.enum_interface_info();
    assert_eq!(interfaces.len(), 1);

    let log_handle = log_factory
        .init(
            Some(Dict::new(vec![
                ("log.timestamp".to_string(), "local".to_string()),
                ("log.level".to_string(), "7".to_string()),
                ("log.line".to_string(), true.to_string()),
            ])),
            &support,
        )
        .expect("Log factory loading should succeed");

    let log_iface = log_handle
        .get_interface(interface::LOG)
        .expect("Log factory should produce an interface");

    let log = log_iface
        .downcast_box::<LogImpl>()
        .expect("Log interface should be a LogImpl");

    let log_topic = interface::log::LogTopic {
        topic: c"test.topic",
        level: LogLevel::Debug,
        has_custom_level: true,
    };

    log.logt(
        LogLevel::Error,
        &log_topic,
        c"file_name.rs",
        123,
        c"function_name",
        format_args!("log test: {}", "some format"),
    );

    support.add_interface(interface::LOG, log);

    let system_factory = plugin
        .find_factory(interface::plugin::SYSTEM_FACTORY)
        .expect("Should find system factory");

    let interfaces = system_factory.enum_interface_info();
    assert_eq!(interfaces.len(), 1);

    let system_handle = system_factory
        .init(None, &support)
        .expect("System factory loading should succeed");

    let system_iface = system_handle
        .get_interface(interface::SYSTEM)
        .expect("System factory should produce an interface");

    let system = system_iface
        .downcast_box::<SystemImpl>()
        .expect("System interface should be a SystemImpl");

    support.add_interface(interface::SYSTEM, system);

    let cpu_factory = plugin
        .find_factory(interface::plugin::CPU_FACTORY)
        .expect("Should find cpu factory");

    let interfaces = cpu_factory.enum_interface_info();
    assert_eq!(interfaces.len(), 1);

    let cpu_handle = cpu_factory
        .init(None, &support)
        .expect("CPU factory loading should succeed");

    let cpu_iface = cpu_handle
        .get_interface(interface::CPU)
        .expect("CPU factory should produce an interface");

    let cpu = cpu_iface
        .downcast_box::<CpuImpl>()
        .expect("CPU interface should be a CpuImpl");

    support.add_interface(interface::CPU, cpu);

    let loop_factory = plugin
        .find_factory(interface::plugin::LOOP_FACTORY)
        .expect("Should find loop factory");

    let interfaces = loop_factory.enum_interface_info();
    assert_eq!(interfaces.len(), 3);

    let loop_handle = loop_factory
        .init(None, &support)
        .expect("Loop factory loading should succeed");

    let loop_iface = loop_handle
        .get_interface(interface::LOOP)
        .expect("Loop factory should produce an interface");

    let r#loop = loop_iface
        .downcast_box::<LoopImpl>()
        .expect("Loop interface should be a LoopImpl");

    support.add_interface(interface::LOOP, r#loop);

    let loop_ctrl_iface = loop_handle
        .get_interface(interface::LOOP_CONTROL)
        .expect("Loop factory should produce control interface");

    let loop_ctrl = loop_ctrl_iface
        .downcast_box::<ControlMethodsImpl>()
        .expect("Loop control interface should be ControlMethodsImpl");

    support.add_interface(interface::LOOP_CONTROL, loop_ctrl);

    let loop_utils_iface = loop_handle
        .get_interface(interface::LOOP_UTILS)
        .expect("Loop factory should produce utils interface");

    let loop_utils = loop_utils_iface
        .downcast_box::<LoopUtilsImpl>()
        .expect("Loop utils interface should be LoopUtilsImpl");

    support.add_interface(interface::LOOP_UTILS, loop_utils);
}
