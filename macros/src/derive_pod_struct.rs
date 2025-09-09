// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, parse_quote, Data, DeriveInput, Fields, Ident};

pub fn derive_pod_struct(input: TokenStream) -> TokenStream {
    let DeriveInput {
        ident,
        generics,
        data,
        ..
    } = parse_macro_input!(input);

    let data = match data {
        Data::Struct(s) => s,
        _ => return quote! {}.into(),
    };

    let fields = match data.fields {
        Fields::Named(f) => f.named,
        Fields::Unnamed(_) => return quote! {}.into(),
        Fields::Unit => return quote! {}.into(),
    };

    let struct_parser_ident: Ident = parse_quote! { struct_parser };
    let mut fields_encode = vec![];
    let mut fields_decode = vec![];

    /* Each line pushes the field into to the StructBuilder */
    for f in fields {
        let field_name = f.ident.unwrap();
        let field_type = f.ty;

        fields_encode.push(quote! {
            .push_pod(&self.#field_name)
        });

        fields_decode.push(quote! {
            #field_name: #struct_parser_ident.pop_pod::<#field_type>()?,
        });
    }

    quote! {
        impl #generics pipewire_native_spa::pod::Pod for #ident #generics {
            type DecodesTo = #ident #generics;

            fn encode(&self, data: &mut [u8]) -> Result<usize, pipewire_native_spa::pod::Error> {
                let builder = pipewire_native_spa::pod::builder::Builder::new(data);

                builder.push_struct(|struct_builder| {
                    struct_builder
                        #(#fields_encode)*
                })
                .build().map(|res| res.len())
            }

            fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), pipewire_native_spa::pod::Error> {
                let mut parser = pipewire_native_spa::pod::parser::Parser::new(data);

                parser.pop_struct(|#struct_parser_ident| {
                    Ok(Self {
                        #(#fields_decode)*
                    })
                })
            }
        }
    }
    .into()
}
