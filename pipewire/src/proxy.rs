// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use crate::{types::ObjectType, Id};

pub trait Proxy {
    fn type_() -> ObjectType
    where
        Self: Sized;
    fn version() -> u32
    where
        Self: Sized;

    fn id(&self) -> Id;
}
