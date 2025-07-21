// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::any::Any;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::os::fd::RawFd;
use std::pin::Pin;

use crate::interface::plugin::HandleFactory;
use crate::interface::r#loop::{self, InvokeFn, Source};
use crate::interface::r#loop::{LoopImpl, SourceFn};
use crate::interface::system::SystemImpl;
use crate::{flags, interface};

pub struct Loop {
    system: Box<SystemImpl>,
    pollfd: RawFd,
    #[allow(clippy::type_complexity)]
    sources: HashMap<RawFd, (Pin<Box<Source>>, Pin<Box<SourceFn>>)>,
}

impl Loop {
    pub fn new_impl(support: &interface::Support) -> std::io::Result<LoopImpl> {
        let system_iface = super::plugin()
            .init(None, support)
            .unwrap()
            .get_interface(interface::SYSTEM)
            .unwrap();
        let system = system_iface.downcast_box::<SystemImpl>().unwrap();
        let pollfd = system.pollfd_create(flags::Fd::CLOEXEC)?;

        Ok(LoopImpl {
            inner: Box::pin(Self {
                system,
                pollfd,
                sources: HashMap::new(),
            }),

            add_source: Self::add_source,
            update_source: Self::update_source,
            remove_source: Self::remove_source,
            invoke: Self::invoke,
        })
    }
}

impl Loop {
    fn add_source(
        this: &mut LoopImpl,
        source: &r#loop::Source,
        func: Box<SourceFn>,
    ) -> std::io::Result<i32> {
        // Shenanigans until downcast_mut_unchecked() is stable
        let inner = unsafe { Pin::into_inner_unchecked(this.inner.as_mut()) };
        let self_ = unsafe { &mut *(inner as *mut dyn Any as *mut Loop) };

        let fd = source.fd;
        let events =
            flags::Io::from_bits(source.mask).ok_or(Error::from(ErrorKind::InvalidInput))?;
        let mut source_ = Box::pin(*source);
        let data = Pin::into_inner(source_.as_mut()) as *mut Source as u64;

        source_.rmask = 0;
        self_
            .sources
            .insert(source.fd, (source_, Box::into_pin(func)));

        self_.system.pollfd_add(self_.pollfd, fd, events, data)
    }

    fn update_source(this: &mut LoopImpl, source: &r#loop::Source) -> std::io::Result<i32> {
        // Shenanigans until downcast_mut_unchecked() is stable
        let inner = unsafe { Pin::into_inner_unchecked(this.inner.as_mut()) };
        let self_ = unsafe { &mut *(inner as *mut dyn Any as *mut Loop) };

        let fd = source.fd;
        let entry = self_
            .sources
            .get_mut(&fd)
            .ok_or(std::io::Error::from(std::io::ErrorKind::NotFound))?;
        let events =
            flags::Io::from_bits(source.mask).ok_or(Error::from(ErrorKind::InvalidInput))?;
        let data = Pin::into_inner(entry.0.as_mut()) as *mut Source as u64;

        // Update the mask, as requested
        entry.0.mask = source.mask;

        self_.system.pollfd_mod(self_.pollfd, fd, events, data)
    }

    fn remove_source(this: &mut LoopImpl, fd: RawFd) -> std::io::Result<i32> {
        // Shenanigans until downcast_mut_unchecked() is stable
        let inner = unsafe { Pin::into_inner_unchecked(this.inner.as_mut()) };
        let self_ = unsafe { &mut *(inner as *mut dyn Any as *mut Loop) };

        self_.system.pollfd_del(self_.pollfd, fd)?;
        self_.sources.remove(&fd);
        Ok(0)
    }

    fn invoke(
        _this: &LoopImpl,
        _seq: u32,
        _data: &[u8],
        _block: bool,
        _func: Box<InvokeFn>,
    ) -> std::io::Result<i32> {
        Err(Error::from(ErrorKind::NotFound))
    }
}
