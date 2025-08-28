// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

pub(crate) mod client;
pub(crate) mod core;
pub(crate) mod device;
pub(crate) mod factory;
pub(crate) mod link;
pub(crate) mod message;
pub(crate) mod metadata;
pub(crate) mod module;
pub(crate) mod node;
pub(crate) mod port;
pub(crate) mod registry;

use pipewire_native_spa::{self as spa, pod::Pod};

pub(crate) const HEADER_LEN: usize = 16;

pub(crate) trait Marshallable {
    fn opcode(&self) -> u8;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error>;
    fn decode(opcode: u8, data: &[u8]) -> Result<(Self, usize), spa::pod::Error>
    where
        Self: Sized;
}

// Container for an arbitrary list of pairs (dict, or hashmaps) that we want to be able to
// serialise into a struct in the form:
//
//  Struct(
//      Int: n_items
//      (K: key
//       V: value)*
//  )
#[derive(Debug)]
pub(crate) struct PairList<K: Pod<DecodesTo = K>, V: Pod<DecodesTo = V>> {
    pub(crate) data: Vec<(K, V)>,
}

impl<K: Pod<DecodesTo = K>, V: Pod<DecodesTo = V>> PairList<K, V> {
    fn new(data: Vec<(K, V)>) -> Self {
        Self { data }
    }
}

impl<K: Pod<DecodesTo = K>, V: Pod<DecodesTo = V>> Pod for PairList<K, V> {
    type DecodesTo = Self;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error> {
        let builder = spa::pod::builder::Builder::new(data);

        let out = builder
            .push_struct(|sb| {
                let mut sb = sb.push_int(self.data.len() as i32);

                for item in &self.data {
                    sb = sb.push_pod(&item.0);
                    sb = sb.push_pod(&item.1);
                }

                sb
            })
            .build()?;

        Ok(out.len())
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), spa::pod::Error> {
        let mut parser = spa::pod::parser::Parser::new(data);

        parser.pop_struct(|sp| {
            let n_items = sp.pop_int()?;
            let mut items = Vec::with_capacity(n_items as usize);

            for _ in 0..n_items {
                let key = sp.pop_pod::<K>()?;
                let value = sp.pop_pod::<V>()?;
                items.push((key, value));
            }

            Ok(Self { data: items })
        })
    }
}
