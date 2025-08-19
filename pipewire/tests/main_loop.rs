// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Sanchayan Maity

use pipewire_native::main_loop::{MainLoop, MainLoopEvents};
use pipewire_native::properties::Properties;
use pipewire_native_spa::flags;

use serial_test::serial;
use std::collections::HashMap;
use std::io::{pipe, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

const IO_CB: &str = "IO";
const EVENT_CB: &str = "EVENT";
const TIMER_CB: &str = "TIMER";
const IDLE_CB: &str = "IDLE";
const LISTENER_CB: &str = "LISTENER";
const LOOP_TIMEOUT: Duration = Duration::from_secs(5);

static CALLBACKS: LazyLock<Mutex<HashMap<String, bool>>> = LazyLock::new(|| HashMap::new().into());

#[allow(dead_code)]
enum MainLoopRun {
    Run,
    Iterate,
}

fn test_mainloop(exec: MainLoopRun) {
    pipewire_native::init();

    let v: Vec<(String, String)> = vec![("loop.name".to_string(), "pw-main-loop".to_string())];
    let ml = MainLoop::new(&Properties::new_vec(v)).unwrap();

    let fd = ml.get_fd();
    assert!(fd != 0);

    let (reader, mut writer) = pipe().unwrap();
    let rx_fd = reader.as_raw_fd();

    let io_src = ml.add_io(rx_fd, flags::Io::IN, false, Box::new(io_callback));
    assert!(io_src.is_some());
    let mut io_src = io_src.unwrap();

    let res = ml.update_io(&mut io_src, flags::Io::IN);
    assert!(res.is_ok());

    writer.write("Hello".as_bytes()).unwrap();

    let event_src = ml.add_event(Box::new(event_callback));
    assert!(event_src.is_some());
    let mut event_src = event_src.unwrap();

    let res = ml.signal_event(&mut event_src);
    assert!(res.is_ok());

    let timer_src = ml.add_timer(Box::new(timer_callback));
    assert!(timer_src.is_some());
    let mut timer_src = timer_src.unwrap();

    let timeout = libc::timespec {
        tv_sec: 1,
        tv_nsec: 0,
    };
    let res = ml.update_timer(&mut timer_src, &timeout, None, true);
    assert!(res.is_ok());

    let idle_src = ml.add_idle(false, Box::new(idle_callback));
    assert!(idle_src.is_some());
    let mut idle_src = idle_src.unwrap();

    let res = ml.enable_idle(&mut idle_src, true);
    assert!(res.is_ok());

    ml.add_listener(MainLoopEvents {
        destroy: Some(Box::new(listener_callback)),
    });

    // Validate that our callbacks have not been called yet (should only happen on run())
    let cb = CALLBACKS.lock().unwrap();
    assert_eq!(cb.get(IO_CB), None);
    assert_eq!(cb.get(EVENT_CB), None);
    assert_eq!(cb.get(TIMER_CB), None);
    assert_eq!(cb.get(IDLE_CB), None);
    assert_eq!(cb.get(LISTENER_CB), None);
    drop(cb);

    match exec {
        MainLoopRun::Run => {
            let ml_ = ml.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(1));
                ml_.quit();
            });
            ml.run();
        }
        MainLoopRun::Iterate => {
            ml.enter();
            assert_eq!(ml.check().ok().unwrap(), 1);
            let methods_dispatched = ml.iterate(Some(LOOP_TIMEOUT));
            /*
             * Four methods should have been dispatched as per above
             * - IO
             * - Signal
             * - Timer
             * - Idle
             */
            assert_eq!(methods_dispatched.ok().unwrap(), 4);
            ml.leave();
        }
    }

    // Validate that our callbacks were called
    let cb = CALLBACKS.lock().unwrap();
    assert_eq!(cb.get(IO_CB).unwrap(), &true);
    assert_eq!(cb.get(EVENT_CB).unwrap(), &true);
    assert_eq!(cb.get(TIMER_CB).unwrap(), &true);
    assert_eq!(cb.get(IDLE_CB).unwrap(), &true);
    assert_eq!(cb.get(LISTENER_CB), None);
    drop(cb);

    ml.destroy_source(io_src);
    ml.destroy_source(event_src);
    ml.destroy_source(timer_src);
    ml.destroy_source(idle_src);
    drop(ml);

    let mut cb = CALLBACKS.lock().unwrap();
    assert_eq!(cb.get(LISTENER_CB).unwrap(), &true);
    cb.clear();
}

#[test]
#[serial]
fn test_main_loop_iterate() {
    test_mainloop(MainLoopRun::Iterate);
}

#[test]
#[serial]
fn test_main_loop_run() {
    test_mainloop(MainLoopRun::Run);
}

fn listener_callback() {
    CALLBACKS
        .lock()
        .unwrap()
        .insert(LISTENER_CB.to_string(), true);
}

fn io_callback(_fd: RawFd, mask: u32) {
    assert_eq!(mask, flags::Io::IN.bits());
    CALLBACKS.lock().unwrap().insert(IO_CB.to_string(), true);
}

fn idle_callback() {
    CALLBACKS.lock().unwrap().insert(IDLE_CB.to_string(), true);
}

fn event_callback(count: u64) {
    assert_eq!(count, 1);
    CALLBACKS.lock().unwrap().insert(EVENT_CB.to_string(), true);
}

fn timer_callback(expirations: u64) {
    assert_eq!(expirations, 1);
    CALLBACKS.lock().unwrap().insert(TIMER_CB.to_string(), true);
}
