// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use proc_macro::TokenStream;

mod derive_enum_u32;
mod derive_marshallable;
mod derive_pod_struct;

#[proc_macro_derive(EnumU32)]
pub fn proc_macro_enum_u32(item: TokenStream) -> TokenStream {
    derive_enum_u32::derive_enum_u32(item)
}

#[proc_macro_derive(PodStruct)]
pub fn proc_macro_pod_struct(item: TokenStream) -> TokenStream {
    derive_pod_struct::derive_pod_struct(item)
}

#[proc_macro_derive(Marshallable, attributes(opcode))]
pub fn proc_macro_marshallable(item: TokenStream) -> TokenStream {
    derive_marshallable::derive_marshallable(item)
}
