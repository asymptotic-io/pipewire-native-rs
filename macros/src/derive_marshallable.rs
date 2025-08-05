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

    let mut encodes = vec![];
    let mut decodes = vec![];

    let mut discriminant: Expr = parse_quote!(0);

    /* Each line pushes the field into to the StructBuilder */
    for v in data.variants {
        let variant_name = v.ident;

        discriminant = if let Some(d) = v.discriminant {
            d.1
        } else {
            discriminant
        };

        assert!(v.fields.len() == 1);
        let variant_inner = v.fields.iter().next().unwrap();
        let variant_inner_type = &variant_inner.ty;

        encodes.push(quote! {
            Self::#variant_name(o) => o.encode(#data_ident),
        });

        decodes.push(quote! {
            if opcode == #discriminant {
                return #variant_inner_type::decode(#data_ident).map(|(o, s)| {
                    (Self::#variant_name(o), s)
                })
            }
        });

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
                    #(#encodes)*
                }
            }

            fn decode(opcode: u8, data: &[u8]) -> Result<(Self, usize), pipewire_native_spa::pod::Error>
            where
                Self:Sized
            {
                #(#decodes)*

                Err(pipewire_native_spa::pod::Error::Invalid)
            }
        }
    }
    .into()
}
