// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::path::PathBuf;

use pipewire_native_spa::dict::Dict;
use pipewire_native_spa::interface;
use pipewire_native_spa::interface::cpu::CpuImpl;
use pipewire_native_spa::interface::log::{LogImpl, LogLevel};
use pipewire_native_spa::interface::r#loop::{LoopControlMethodsImpl, LoopImpl, LoopUtilsImpl};
use pipewire_native_spa::interface::system::SystemImpl;
use pipewire_native_spa::support::ffi;
use std::time::Duration;

const LOOP_TIMEOUT: Duration = Duration::from_secs(5);

fn init_support() -> (interface::Support, ffi::plugin::Plugin) {
    let plugin_path = std::env::var("SPA_TEST_PLUGIN_PATH")
        .unwrap_or("/usr/lib64/spa-0.2/support/libspa-support.so".to_string());

    let plugin =
        ffi::plugin::load(&PathBuf::from(plugin_path)).expect("Plugin loading should not fail");

    let support = interface::Support::new();

    (support, plugin)
}

fn setup_log(
    support: &mut interface::Support,
    plugin: &ffi::plugin::Plugin,
) -> Box<dyn interface::plugin::Handle> {
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
            support,
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

    log_handle
}

fn setup_system(
    support: &mut interface::Support,
    plugin: &ffi::plugin::Plugin,
) -> Box<dyn interface::plugin::Handle> {
    let system_factory = plugin
        .find_factory(interface::plugin::SYSTEM_FACTORY)
        .expect("Should find system factory");

    let interfaces = system_factory.enum_interface_info();
    assert_eq!(interfaces.len(), 1);

    let system_handle = system_factory
        .init(None, support)
        .expect("System factory loading should succeed");

    let system_iface = system_handle
        .get_interface(interface::SYSTEM)
        .expect("System factory should produce an interface");

    let system = system_iface
        .downcast_box::<SystemImpl>()
        .expect("System interface should be a SystemImpl");

    support.add_interface(interface::SYSTEM, system);

    system_handle
}

fn setup_cpu(
    support: &mut interface::Support,
    plugin: &ffi::plugin::Plugin,
) -> Box<dyn interface::plugin::Handle> {
    let cpu_factory = plugin
        .find_factory(interface::plugin::CPU_FACTORY)
        .expect("Should find cpu factory");

    let interfaces = cpu_factory.enum_interface_info();
    assert_eq!(interfaces.len(), 1);

    let cpu_handle = cpu_factory
        .init(None, support)
        .expect("CPU factory loading should succeed");

    let cpu_iface = cpu_handle
        .get_interface(interface::CPU)
        .expect("CPU factory should produce an interface");

    let cpu = cpu_iface
        .downcast_box::<CpuImpl>()
        .expect("CPU interface should be a CpuImpl");

    support.add_interface(interface::CPU, cpu);

    cpu_handle
}

fn setup_loop(
    support: &mut interface::Support,
    plugin: &ffi::plugin::Plugin,
) -> Box<dyn interface::plugin::Handle> {
    let loop_factory = plugin
        .find_factory(interface::plugin::LOOP_FACTORY)
        .expect("Should find loop factory");

    let interfaces = loop_factory.enum_interface_info();
    assert_eq!(interfaces.len(), 3);

    let loop_handle = loop_factory
        .init(None, support)
        .expect("Loop factory loading should succeed");

    let loop_iface = loop_handle
        .get_interface(interface::LOOP)
        .expect("Loop factory should produce an interface");

    let r#loop = loop_iface
        .downcast_box::<LoopImpl>()
        .expect("Loop interface should be a LoopImpl");

    support.add_interface(interface::LOOP, r#loop);

    loop_handle
}

fn setup_loop_ctrl(
    support: &mut interface::Support,
    loop_handle: &Box<dyn interface::plugin::Handle>,
) {
    let loop_ctrl_iface = loop_handle
        .get_interface(interface::LOOP_CONTROL)
        .expect("Loop factory should produce control interface");

    let loop_ctrl = loop_ctrl_iface
        .downcast_box::<LoopControlMethodsImpl>()
        .expect("Loop control interface should be LoopControlMethodsImpl");

    support.add_interface(interface::LOOP_CONTROL, loop_ctrl);
}

fn setup_loop_utils(
    support: &mut interface::Support,
    loop_handle: &Box<dyn interface::plugin::Handle>,
) {
    let loop_utils_iface = loop_handle
        .get_interface(interface::LOOP_UTILS)
        .expect("Loop factory should produce utils interface");

    let loop_utils = loop_utils_iface
        .downcast_box::<LoopUtilsImpl>()
        .expect("Loop utils interface should be LoopUtilsImpl");

    support.add_interface(interface::LOOP_UTILS, loop_utils);
}

#[test]
fn test_load_support() {
    let (mut support, plugin) = init_support();

    let _log_handle = setup_log(&mut support, &plugin);
    let _system_handle = setup_system(&mut support, &plugin);
    let _cpu_handle = setup_cpu(&mut support, &plugin);
    let loop_handle = setup_loop(&mut support, &plugin);

    setup_loop_ctrl(&mut support, &loop_handle);
    setup_loop_utils(&mut support, &loop_handle);
}

#[test]
fn test_loop_support() {
    let (mut support, plugin) = init_support();

    let _log_handle = setup_log(&mut support, &plugin);
    let _system_handle = setup_system(&mut support, &plugin);
    let _cpu_handle = setup_cpu(&mut support, &plugin);
    let loop_handle = setup_loop(&mut support, &plugin);

    setup_loop_ctrl(&mut support, &loop_handle);
    setup_loop_utils(&mut support, &loop_handle);

    let eloop = support.get_interface::<interface::r#loop::LoopImpl>(interface::LOOP);
    let lutils = support.get_interface::<interface::r#loop::LoopUtilsImpl>(interface::LOOP_UTILS);
    let lctrl =
        support.get_interface::<interface::r#loop::LoopControlMethodsImpl>(interface::LOOP_CONTROL);

    if let (Some(_eloop), Some(utils), Some(ctrl)) = (eloop, lutils, lctrl) {
        let fd = ctrl.get_fd();
        assert!(fd != 0);

        let event_src = utils.add_event(Box::new(event_callback));
        assert!(event_src.is_some());
        let mut event_src = event_src.unwrap();

        let res = utils.signal_event(&mut event_src);
        assert!(res.is_ok());

        let timer_src = utils.add_timer(Box::new(timer_callback));
        assert!(timer_src.is_some());
        let mut timer_src = timer_src.unwrap();

        let timeout = libc::timespec {
            tv_sec: 1,
            tv_nsec: 0,
        };
        let res = utils.update_timer(&mut timer_src, &timeout, None, true);
        assert!(res.is_ok());

        let idle_src = utils.add_idle(false, Box::new(idle_callback));
        assert!(idle_src.is_some());
        let mut idle_src = idle_src.unwrap();

        let res = utils.enable_idle(&mut idle_src, true);
        assert!(res.is_ok());

        ctrl.enter();
        assert_eq!(ctrl.check(), 1);
        let methods_dispatched = ctrl.iterate(Some(LOOP_TIMEOUT));
        /*
         * Three methods should have been dispatched as per above
         * - Signal
         * - Timer
         * - Idle
         */
        assert_eq!(methods_dispatched, 3);
        ctrl.leave();

        utils.destroy_source(event_src);
        utils.destroy_source(timer_src);
        utils.destroy_source(idle_src);
    } else {
        panic!("Failed to get loop control or utils");
    }
}

fn idle_callback() {
    println!("Idle callback");
}

fn event_callback(count: u64) {
    println!("Event callback, count: {count}");
}

fn timer_callback(expirations: u64) {
    println!("Timer callback, expirations: {expirations}");
}
