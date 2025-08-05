// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, BinOp, Data, DeriveInput, Expr, ExprBinary, Ident};

pub fn derive_marshallable(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        generics,
        data,
        ..
    } = parse_macro_input!(input);

    let data = match data {
        Data::Enum(s) => s,
        _ => return quote! {}.into(),
    };

    let data_ident: Ident = parse_quote!(data);

    let mut variant_names = vec![];
    let mut variant_types = vec![];
    let mut discriminants = vec![];

    let mut discriminant: Expr = parse_quote!(0);

    for v in data.variants {
        discriminant = if let Some(d) = v.discriminant {
            d.1
        } else {
            discriminant
        };

        assert!(v.fields.len() == 1);
        let variant_inner = v.fields.iter().next().unwrap();
        let variant_inner_type = &variant_inner.ty;

        variant_names.push(v.ident);
        variant_types.push(variant_inner_type.clone());
        discriminants.push(discriminant.clone());

        discriminant = Expr::from(ExprBinary {
            attrs: vec![],
            left: Box::new(discriminant),
            op: BinOp::Add(parse_quote! { + }),
            right: Box::new(parse_quote! { 1 }),
        });
    }

    quote! {
        impl #generics crate::protocol::marshal::Marshallable for #ident {
            fn encode(&self, data: &mut [u8]) -> Result<usize, pipewire_native_spa::pod::Error> {
                match self {
                #(
                    Self::#variant_names(o) => o.encode(#data_ident),
                )*
                }
            }

            fn decode(opcode: u8, data: &[u8]) -> Result<(Self, usize), pipewire_native_spa::pod::Error>
            where
                Self:Sized
            {
                #(
                    if opcode == #discriminants {
                        return #variant_types::decode(#data_ident).map(|(o, s)| {
                            (Self::#variant_names(o), s)
                        })
                    }
                )*

                Err(pipewire_native_spa::pod::Error::Invalid)
            }
        }
    }
    .into()
}
