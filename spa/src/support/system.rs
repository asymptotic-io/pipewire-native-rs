// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{io::Error, os::fd::RawFd};

use crate::interface::system::{result_or_error, PollEvent, PollEvents, SystemImpl};

struct System {}

pub fn new() -> SystemImpl {
    SystemImpl {
        inner: Box::pin(System {}),

        read: System::read,
        write: System::write,
        ioctl: libc::ioctl,
        close: System::close,

        clock_gettime: System::clock_gettime,
        clock_getres: System::clock_getres,

        pollfd_create: System::pollfd_create,
        pollfd_add: System::pollfd_add,
        pollfd_mod: System::pollfd_mod,
        pollfd_del: System::pollfd_del,
        pollfd_wait: System::pollfd_wait,

        timerfd_create: System::timerfd_create,
        timerfd_settime: System::timerfd_settime,
        timerfd_gettime: System::timerfd_gettime,
        timerfd_read: System::timerfd_read,

        eventfd_create: System::eventfd_create,
        eventfd_read: System::eventfd_read,
        eventfd_write: System::eventfd_write,

        signalfd_create: System::signalfd_create,
        signalfd_read: System::signalfd_read,
    }
}

impl System {
    fn read(_this: &SystemImpl, fd: RawFd, buf: &mut [u8]) -> std::io::Result<isize> {
        let res = unsafe { libc::read(fd, buf.as_ptr() as *mut libc::c_void, buf.len()) };
        result_or_error(res)
    }

    fn write(_this: &SystemImpl, fd: RawFd, buf: &[u8]) -> std::io::Result<isize> {
        let res = unsafe { libc::write(fd, buf.as_ptr() as *const libc::c_void, buf.len()) };
        result_or_error(res)
    }

    fn close(_this: &SystemImpl, fd: RawFd) -> std::io::Result<i32> {
        let res = unsafe { libc::close(fd) };
        result_or_error(res)
    }

    fn pollfd_create(_this: &SystemImpl, flags: i32) -> std::io::Result<i32> {
        let res = unsafe { libc::epoll_create1(flags) };
        result_or_error(res)
    }

    fn clock_gettime(
        _this: &SystemImpl,
        clockid: libc::clockid_t,
        res: &mut libc::timespec,
    ) -> std::io::Result<i32> {
        let res = unsafe { libc::clock_gettime(clockid, res) };
        result_or_error(res)
    }

    fn clock_getres(
        _this: &SystemImpl,
        clockid: libc::clockid_t,
        res: &mut libc::timespec,
    ) -> std::io::Result<i32> {
        let res = unsafe { libc::clock_getres(clockid, res) };
        result_or_error(res)
    }

    fn pollfd_add(
        _this: &SystemImpl,
        pfd: RawFd,
        fd: RawFd,
        events: PollEvents,
        data: u64,
    ) -> std::io::Result<i32> {
        let mut event = libc::epoll_event {
            events: events.bits(),
            u64: data,
        };
        let res = unsafe {
            libc::epoll_ctl(
                pfd,
                libc::EPOLL_CTL_ADD,
                fd,
                &mut event as *mut libc::epoll_event,
            )
        };
        result_or_error(res)
    }

    fn pollfd_mod(
        _this: &SystemImpl,
        pfd: RawFd,
        fd: RawFd,
        events: PollEvents,
        data: u64,
    ) -> std::io::Result<i32> {
        let mut event = libc::epoll_event {
            events: events.bits(),
            u64: data,
        };
        let res = unsafe {
            libc::epoll_ctl(
                pfd,
                libc::EPOLL_CTL_MOD,
                fd,
                &mut event as *mut libc::epoll_event,
            )
        };
        result_or_error(res)
    }

    fn pollfd_del(_this: &SystemImpl, pfd: RawFd, fd: RawFd) -> std::io::Result<i32> {
        let res = unsafe {
            libc::epoll_ctl(
                pfd,
                libc::EPOLL_CTL_DEL,
                fd,
                std::ptr::null_mut::<libc::epoll_event>(),
            )
        };
        result_or_error(res)
    }

    fn pollfd_wait(
        _this: &SystemImpl,
        pfd: RawFd,
        events: &mut [PollEvent],
        timeout: i32,
    ) -> std::io::Result<i32> {
        let res = unsafe {
            libc::epoll_wait(
                pfd,
                events.as_mut_ptr() as *mut libc::epoll_event,
                events.len() as i32,
                timeout,
            )
        };
        result_or_error(res)
    }

    fn timerfd_create(_this: &SystemImpl, clockid: i32, flags: i32) -> std::io::Result<i32> {
        let res = unsafe { libc::timerfd_create(clockid, flags) };
        result_or_error(res)
    }

    fn timerfd_settime(
        _this: &SystemImpl,
        fd: RawFd,
        flags: i32,
        new_value: &libc::itimerspec,
        old_value: &mut libc::itimerspec,
    ) -> std::io::Result<i32> {
        let res = unsafe { libc::timerfd_settime(fd, flags, new_value, old_value) };
        result_or_error(res)
    }

    pub fn timerfd_gettime(
        _this: &SystemImpl,
        fd: RawFd,
        curr_value: &mut libc::itimerspec,
    ) -> std::io::Result<i32> {
        let res = unsafe { libc::timerfd_gettime(fd, curr_value) };
        result_or_error(res)
    }

    pub fn timerfd_read(_this: &SystemImpl, fd: RawFd) -> std::io::Result<u64> {
        let mut buf = 0u64;
        let res = unsafe { libc::read(fd, &mut buf as *mut u64 as *mut libc::c_void, 8) };
        if res < 0 {
            Err(Error::last_os_error())
        } else if res != 8 {
            Err(Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "read timerfd returned unexpected size",
            ))
        } else {
            Ok(buf)
        }
    }

    pub fn eventfd_create(_this: &SystemImpl, flags: i32) -> std::io::Result<i32> {
        let res = unsafe { libc::eventfd(0, flags) };
        result_or_error(res)
    }

    pub fn eventfd_read(_this: &SystemImpl, fd: RawFd) -> std::io::Result<u64> {
        let mut buf = 0u64;
        let res = unsafe { libc::read(fd, &mut buf as *mut u64 as *mut libc::c_void, 8) };
        if res < 0 {
            Err(Error::last_os_error())
        } else if res != 8 {
            Err(Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "read eventfd returned unexpected size",
            ))
        } else {
            Ok(buf)
        }
    }

    pub fn eventfd_write(_this: &SystemImpl, fd: RawFd, value: u64) -> std::io::Result<i32> {
        let res = unsafe { libc::write(fd, &value as *const u64 as *const libc::c_void, 8) };
        if res < 0 {
            Err(Error::last_os_error())
        } else if res != 8 {
            Err(Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "write eventfd returned unexpected size",
            ))
        } else {
            Ok(0)
        }
    }

    pub fn signalfd_create(_this: &SystemImpl, signal: u32, flags: i32) -> std::io::Result<i32> {
        let res = unsafe {
            let mut mask: libc::sigset_t = std::mem::zeroed();

            libc::sigemptyset(&mut mask);
            libc::sigaddset(&mut mask, signal as i32);

            libc::signalfd(-1, &mask, flags)
        };
        result_or_error(res)
    }

    pub fn signalfd_read(_this: &SystemImpl, fd: RawFd) -> std::io::Result<u32> {
        let mut siginfo: libc::signalfd_siginfo = unsafe { std::mem::zeroed() };
        let res = unsafe {
            libc::read(
                fd,
                &mut siginfo as *mut libc::signalfd_siginfo as *mut libc::c_void,
                std::mem::size_of::<libc::signalfd_siginfo>(),
            )
        };
        if res < 0 {
            Err(Error::last_os_error())
        } else if res != std::mem::size_of::<libc::signalfd_siginfo>() as isize {
            Err(Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "read signalfd returned unexpected size",
            ))
        } else {
            Ok(siginfo.ssi_signo)
        }
    }
}

unsafe impl Send for System {}
unsafe impl Sync for System {}
