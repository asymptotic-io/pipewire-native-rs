// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use std::{
    os::fd::RawFd,
    rc::{Rc, Weak},
};

use crate::{debug, default_topic, log, refcounted};

default_topic!(log::topic::PROTOCOL);

refcounted! {
    pub(crate) struct Connection {
        fd: RawFd,
    }
}

impl Connection {
    pub(crate) fn new(fd: RawFd) -> Self {
        debug!("Creating new connection to {fd}");
        Self {
            inner: Rc::new(InnerConnection::new(fd)),
        }
    }
}

impl InnerConnection {
    pub(crate) fn new(fd: RawFd) -> Self {
        InnerConnection { fd }
    }
}
