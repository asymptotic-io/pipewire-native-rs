// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, BinOp, Data, DeriveInput, Expr, ExprBinary, Ident};

pub fn derive_enum_u32(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        generics,
        data,
        ..
    } = parse_macro_input!(input);

    let data = match data {
        Data::Enum(d) => d,
        _ => return quote! {}.into(),
    };

    let mut try_from_u32_arms: Vec<Expr> = vec![];
    let mut last_value = None;
    // Argument name for try_from()
    let value_ident: Ident = parse_quote! { value };

    for v in data.variants {
        if !v.fields.is_empty() {
            return quote! {}.into();
        }

        // The variant name
        let id = v.ident;
        // The variant value
        let value: Expr = if let Some(d) = v.discriminant {
            // Specified in the enum, just use that
            d.1
        } else if let Some(v) = last_value {
            // Not specified => 1 + previous value
            Expr::from(ExprBinary {
                attrs: vec![],
                left: Box::new(v),
                op: BinOp::Add(parse_quote! { + }),
                right: Box::new(parse_quote! { 1 }),
            })
        } else {
            // Not specified, and first variant => value is 0
            parse_quote! { 0 }
        };

        try_from_u32_arms.push(parse_quote! {
            if (#value_ident == #value) {
                return Ok(Self::#id);
            }
        });
        last_value = Some(value);
    }

    quote! {
        impl #generics From<#ident #generics> for u32 {
            fn from(value: #ident) -> u32 {
                value as u32
            }
        }

        impl #generics TryFrom<u32> for #ident #generics {
            type Error = ();

            fn try_from(#value_ident: u32) -> Result<#ident, ()> {
                #(#try_from_u32_arms)*

                Err(())
            }
        }
    }
    .into()
}
