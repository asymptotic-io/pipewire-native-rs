// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    any::Any,
    ffi::{c_char, c_int, c_void, CStr, CString},
};

use crate::interface::ffi::CInterface;
use crate::interface::log::{LogImpl, LogLevel, LogTopic};

use super::c_string;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct CLogTopic {
    version: u32,
    topic: *const c_char,
    level: LogLevel,
    has_custom_level: bool,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct CLogMethods {
    version: u32,
    log: extern "C" fn(
        object: *mut c_void,
        level: LogLevel,
        file: *const c_char,
        line: c_int,
        func: *const c_char,
        fmt: *const c_char,
        ...
    ),
    logv: *const c_void, /* va_list currently only in nightly */
    logt: extern "C" fn(
        object: *mut c_void,
        level: LogLevel,
        topic: *const CLogTopic,
        file: *const c_char,
        line: c_int,
        func: *const c_char,
        fmt: *const c_char,
        ...
    ),
    logtv: *const c_void, /* va_list currently only in nightly */
}

#[repr(C)]
struct CLog {
    iface: CInterface,
    level: LogLevel,
}

struct CLogImpl {}

pub fn new_impl(interface: *mut CInterface) -> LogImpl {
    let level = unsafe { (interface as *mut CLog).as_ref().unwrap().level };

    LogImpl {
        inner: Box::pin(interface as *mut CLog),
        level,

        log: CLogImpl::log,
        logt: CLogImpl::logt,
    }
}

extern "C" {
    fn c_log_from_impl(impl_: *const c_void, level: LogLevel) -> *mut CLog;
    fn c_log_free(log: *mut CLog);
}

#[no_mangle]
extern "C" fn rust_logt(
    impl_: *mut c_void,
    level: LogLevel,
    topic: *const CLogTopic,
    file: *const c_char,
    line: i32,
    func: *const c_char,
    log: *const c_char,
) {
    let log_impl = unsafe { &mut *(impl_ as *mut LogImpl) };
    let level = LogLevel::try_from(level as u32).unwrap();
    let file = unsafe { CStr::from_ptr(file).to_str().unwrap() };
    let func = unsafe { CStr::from_ptr(func).to_str().unwrap() };
    let log = unsafe { CStr::from_ptr(log).to_str().unwrap() };

    if topic.is_null() {
        log_impl.log(level, file, line, func, format_args!("{}", log));
    } else {
        let topic = unsafe {
            let c_topic = &*topic;
            LogTopic {
                topic: CStr::from_ptr(c_topic.topic).to_str().unwrap().to_string(),
                level: LogLevel::try_from(c_topic.level as u32).unwrap(),
                has_custom_level: c_topic.has_custom_level,
            }
        };
        log_impl.logt(level, &topic, file, line, func, format_args!("{}", log));
    }
}

pub fn make_native(log: &LogImpl) -> *mut CInterface {
    unsafe { c_log_from_impl(log as *const dyn Any as *const c_void, log.level) as *mut CInterface }
}

pub fn free_native(c_log: *mut CInterface) {
    unsafe {
        c_log_free(c_log as *mut CLog);
    }
}

impl CLogImpl {
    fn log(
        this: &LogImpl,
        level: crate::interface::log::LogLevel,
        file: &str,
        line: i32,
        func: &str,
        args: std::fmt::Arguments,
    ) {
        let log_line = match args.as_str() {
            Some(s) => s,
            _ => return,
        };

        unsafe {
            let log = this
                .inner
                .as_ref()
                .downcast_ref::<*mut CLog>()
                .unwrap()
                .as_ref()
                .unwrap();
            let funcs = log.iface.cb.funcs as *const CLogMethods;
            ((*funcs).log)(
                log.iface.cb.data,
                level,
                c_string(file).as_ptr(),
                line,
                c_string(func).as_ptr(),
                c_string(log_line).as_ptr(),
            )
        };
    }

    fn logt(
        this: &LogImpl,
        level: LogLevel,
        topic: &crate::interface::log::LogTopic,
        file: &str,
        line: i32,
        func: &str,
        args: std::fmt::Arguments,
    ) {
        let topic_name = c_string(topic.topic.as_str());
        let ctopic = CLogTopic {
            version: 0,
            topic: topic_name.as_ptr(),
            level,
            has_custom_level: topic.has_custom_level,
        };
        let log_line = args
            .as_str()
            .map(c_string)
            .unwrap_or(CString::new(args.to_string()).unwrap());

        unsafe {
            let log = this
                .inner
                .as_ref()
                .downcast_ref::<*mut CLog>()
                .unwrap()
                .as_ref()
                .unwrap();
            let funcs = log.iface.cb.funcs as *const CLogMethods;

            ((*funcs).logt)(
                log.iface.cb.data,
                level,
                &ctopic,
                c_string(file).as_ptr(),
                line,
                c_string(func).as_ptr(),
                log_line.as_ptr(),
            )
        };
    }
}
