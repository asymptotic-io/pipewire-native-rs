use pipewire_native_spa as spa;
use spa::dict::Dict;
use spa::flags;
use spa::interface;
use spa::interface::cpu::CpuImpl;
use spa::interface::ffi::{CControlHooks, CHook};
use spa::interface::log::{LogImpl, LogLevel};
use spa::interface::r#loop::{
    LoopControlMethodsImpl, LoopImpl, LoopUtilsImpl, LoopUtilsSource, SourceEventFn, SourceIdleFn,
    SourceIoFn, SourceSignalFn, SourceTimerFn,
};
use spa::interface::system::SystemImpl;
use spa::support::ffi;
use spa::{emit_hook, hook::HookList};

use std::ops::Deref;
use std::os::fd::RawFd;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, OnceLock, Weak,
};

#[allow(dead_code)]
struct Handles {
    log_handle: Box<dyn interface::plugin::Handle + Send + Sync>,
    system_handle: Box<dyn interface::plugin::Handle + Send + Sync>,
    cpu_handle: Box<dyn interface::plugin::Handle + Send + Sync>,
    loop_handle: Box<dyn interface::plugin::Handle + Send + Sync>,
    support: interface::Support,
    plugin: ffi::plugin::Plugin,
}

static HANDLES: OnceLock<Mutex<Handles>> = OnceLock::new();

pub struct MainLoopEvents {
    destroy: Box<dyn FnMut()>,
}

unsafe impl Send for MainLoopEvents {}
unsafe impl Sync for MainLoopEvents {}

#[allow(dead_code)]
#[derive(Clone)]
struct Loop {
    system: Arc<Pin<Box<SystemImpl>>>,
    r#loop: Arc<Pin<Box<LoopImpl>>>,
    control: Arc<Pin<Box<LoopControlMethodsImpl>>>,
    utils: Arc<Pin<Box<LoopUtilsImpl>>>,
    name: String,
}

pub struct MainLoop {
    inner: Arc<InnerMainLoop>,
    running: Arc<AtomicBool>,
}
pub struct WeakMainLoop(Weak<InnerMainLoop>);

impl Deref for MainLoop {
    type Target = InnerMainLoop;

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl WeakMainLoop {
    pub fn upgrade(&self, running: Arc<AtomicBool>) -> Option<MainLoop> {
        self.0.upgrade().map(|l| MainLoop { inner: l, running })
    }
}

impl MainLoop {
    pub fn downgrade(&self) -> (WeakMainLoop, Arc<AtomicBool>) {
        (
            WeakMainLoop(Arc::downgrade(&self.inner)),
            self.running.clone(),
        )
    }

    pub fn new(props: &Dict) -> Option<MainLoop> {
        let Some(l) = InnerMainLoop::new(props) else {
            return None;
        };

        Some(MainLoop {
            inner: Arc::new(l),
            running: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn run(&mut self) {
        let running = self.running.clone();
        self.inner.run(running);
    }

    pub fn quit(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.inner.quit();
    }

    // TODO: Should this just move to Drop?
    pub fn destroy(self) {
        <InnerMainLoop as Clone>::clone(&self.inner).destroy();
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub struct InnerMainLoop {
    pw_loop: Loop,
    hooks: Arc<Mutex<HookList<MainLoopEvents>>>,
}

impl InnerMainLoop {
    pub fn new(props: &Dict) -> Option<InnerMainLoop> {
        let (mut support, plugin) = get_support();

        let log_handle = setup_log(&mut support, &plugin);
        let system_handle = setup_system(&mut support, &plugin);
        let cpu_handle = setup_cpu(&mut support, &plugin);
        let loop_handle = setup_loop(&mut support, &plugin);

        setup_loop_ctrl(&mut support, &loop_handle);
        setup_loop_utils(&mut support, &loop_handle);

        let Some(system) =
            support.get_interface::<interface::system::SystemImpl>(interface::SYSTEM)
        else {
            return None;
        };

        let Some(lloop) = support.get_interface::<interface::r#loop::LoopImpl>(interface::LOOP)
        else {
            return None;
        };

        let Some(lutils) =
            support.get_interface::<interface::r#loop::LoopUtilsImpl>(interface::LOOP_UTILS)
        else {
            return None;
        };

        let Some(lctrl) = support
            .get_interface::<interface::r#loop::LoopControlMethodsImpl>(interface::LOOP_CONTROL)
        else {
            return None;
        };

        let name = if let Some(n) = props.lookup("loop.name") {
            n.to_string()
        } else {
            "main.loop".to_string()
        };

        let handles = Handles {
            log_handle,
            system_handle,
            cpu_handle,
            loop_handle,
            support,
            plugin,
        };

        if let Err(_) = HANDLES.set(Mutex::new(handles)) {
            return None;
        }

        Some(InnerMainLoop {
            pw_loop: Loop {
                system,
                r#loop: lloop,
                control: lctrl,
                utils: lutils,
                name,
            },
            hooks: HookList::new(),
        })
    }

     fn destroy(self) {
        emit_hook!(self.hooks, destroy,);
    }

    pub fn add_listener(&self, events: MainLoopEvents) {
        self.hooks.lock().unwrap().append(events);
    }

    fn run(&self, running: Arc<AtomicBool>) {
        assert_eq!(
            running.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed),
            Ok(false)
        );

        self.pw_loop.control.enter();

        while running.load(Ordering::Relaxed) {
            if let Err(res) = self.pw_loop.control.iterate(Some(std::time::Duration::MAX)) {
                if res.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
            }
        }

        self.pw_loop.control.leave();
    }

    fn quit(&self) {
        // let stop = move |_block: bool, _seq: u32, _data: &[u8]| 0;
        // let _ = self.pw_loop.r#loop.invoke(1, &[], false, Box::new(stop));
    }

    // Loop control methods
    pub fn get_fd(&self) -> u32 {
        self.pw_loop.control.get_fd()
    }

    pub fn add_hook(&self, hook: &CHook, hooks: &CControlHooks, data: u64) {
        self.pw_loop.control.add_hook(hook, hooks, data)
    }

    pub fn enter(&self) {
        self.pw_loop.control.enter()
    }

    pub fn leave(&self) {
        self.pw_loop.control.leave()
    }

    pub fn iterate(&self, timeout: Option<std::time::Duration>) -> std::io::Result<i32> {
        self.pw_loop.control.iterate(timeout)
    }

    pub fn check(&self) -> std::io::Result<i32> {
        self.pw_loop.control.check()
    }

    pub fn lock(&self) -> std::io::Result<i32> {
        self.pw_loop.control.lock()
    }

    pub fn unlock(&self) -> std::io::Result<i32> {
        self.pw_loop.control.unlock()
    }

    pub fn get_time(&self, timeout: std::time::Duration) -> std::io::Result<libc::timespec> {
        self.pw_loop.control.get_time(timeout)
    }

    pub fn wait(&self, abstime: &libc::timespec) -> std::io::Result<i32> {
        self.pw_loop.control.wait(abstime)
    }

    pub fn signal(&self, wait_for_accept: bool) -> std::io::Result<i32> {
        self.pw_loop.control.signal(wait_for_accept)
    }

    pub fn accept(&self) -> std::io::Result<i32> {
        self.pw_loop.control.accept()
    }

    // Loop utils
    pub fn add_io(
        &self,
        fd: RawFd,
        mask: flags::Io,
        close: bool,
        func: Box<SourceIoFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        self.pw_loop.utils.add_io(fd, mask, close, func)
    }

    pub fn update_io(
        &self,
        source: &mut Pin<Box<LoopUtilsSource>>,
        mask: flags::Io,
    ) -> std::io::Result<i32> {
        self.pw_loop.utils.update_io(source, mask)
    }

    pub fn add_idle(
        &self,
        enabled: bool,
        func: Box<SourceIdleFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        self.pw_loop.utils.add_idle(enabled, func)
    }

    pub fn enable_idle(
        &self,
        source: &mut Pin<Box<LoopUtilsSource>>,
        enabled: bool,
    ) -> std::io::Result<i32> {
        self.pw_loop.utils.enable_idle(source, enabled)
    }

    pub fn add_event(&self, func: Box<SourceEventFn>) -> Option<Pin<Box<LoopUtilsSource>>> {
        self.pw_loop.utils.add_event(func)
    }

    pub fn signal_event(&self, source: &mut Pin<Box<LoopUtilsSource>>) -> std::io::Result<i32> {
        self.pw_loop.utils.signal_event(source)
    }

    pub fn add_timer(&self, func: Box<SourceTimerFn>) -> Option<Pin<Box<LoopUtilsSource>>> {
        self.pw_loop.utils.add_timer(func)
    }

    pub fn update_timer(
        &self,
        source: &mut Pin<Box<LoopUtilsSource>>,
        value: &libc::timespec,
        interval: Option<&libc::timespec>,
        absolute: bool,
    ) -> std::io::Result<i32> {
        self.pw_loop
            .utils
            .update_timer(source, value, interval, absolute)
    }

    pub fn add_signal(
        &self,
        signal_number: i32,
        func: Box<SourceSignalFn>,
    ) -> Option<Pin<Box<LoopUtilsSource>>> {
        self.pw_loop.utils.add_signal(signal_number, func)
    }

    pub fn destroy_source(&self, source: Pin<Box<LoopUtilsSource>>) {
        self.pw_loop.utils.destroy_source(source)
    }

    pub fn set_name(&mut self, name: &str) {
        self.pw_loop.name = name.to_string();
    }
}

fn get_support() -> (interface::Support, ffi::plugin::Plugin) {
    let plugin_path = std::env::var("SPA_PLUGIN_PATH")
        .unwrap_or("/usr/lib64/spa-0.2/support/libspa-support.so".to_string());

    let plugin =
        ffi::plugin::load(&PathBuf::from(plugin_path)).expect("Plugin loading should not fail");

    let support = interface::Support::new();

    (support, plugin)
}

fn setup_log(
    support: &mut interface::Support,
    plugin: &ffi::plugin::Plugin,
) -> Box<dyn interface::plugin::Handle + Send + Sync> {
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
) -> Box<dyn interface::plugin::Handle + Send + Sync> {
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
) -> Box<dyn interface::plugin::Handle + Send + Sync> {
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
) -> Box<dyn interface::plugin::Handle + Send + Sync> {
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
    loop_handle: &Box<dyn interface::plugin::Handle + Send + Sync>,
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
    loop_handle: &Box<dyn interface::plugin::Handle + Send + Sync>,
) {
    let loop_utils_iface = loop_handle
        .get_interface(interface::LOOP_UTILS)
        .expect("Loop factory should produce utils interface");

    let loop_utils = loop_utils_iface
        .downcast_box::<LoopUtilsImpl>()
        .expect("Loop utils interface should be LoopUtilsImpl");

    support.add_interface(interface::LOOP_UTILS, loop_utils);
}
