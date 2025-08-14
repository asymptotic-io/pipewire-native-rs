// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

pub(crate) mod client;
pub(crate) mod core;
pub(crate) mod message;
pub(crate) mod registry;

use pipewire_native_spa::{self as spa, pod::Pod};

use message::{Header, Message};

pub(crate) const HEADER_LEN: usize = 16;

pub(crate) trait Marshallable {
    fn opcode(&self) -> u8;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error>;
    fn decode(opcode: u8, data: &[u8]) -> Result<(Self, usize), spa::pod::Error>
    where
        Self: Sized;
}

impl<T: Marshallable, F: Pod<DecodesTo = F>> Pod for Message<T, F> {
    type DecodesTo = Self;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error> {
        // Header + at least one byte for data
        if data.len() < HEADER_LEN + 1 {
            return Err(spa::pod::Error::NoSpace);
        }

        let payload_size = self.object.encode(&mut data[HEADER_LEN..])?;

        let footer_size = if let Some(footer) = &self.footer {
            footer.encode(&mut data[HEADER_LEN + payload_size..])?
        } else {
            0
        };

        let size = payload_size + footer_size;
        let header = Header {
            size: size as u32,
            ..self.header
        };

        header.encode(data)?;

        Ok(HEADER_LEN + size)
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), spa::pod::Error> {
        if data.len() < HEADER_LEN {
            return Err(spa::pod::Error::Invalid);
        }

        let (header, header_size) = Header::decode(data)?;
        let size = header.size as usize;
        let (object, payload_size) = T::decode(header.opcode, &data[header_size..])?;

        let (footer, footer_size) = if size > header_size + payload_size {
            let (f, s) = F::decode(&data[header_size + payload_size..size])?;
            (Some(f), s)
        } else {
            (None, 0)
        };

        if size != header_size + payload_size + footer_size {
            Ok((
                Message {
                    header,
                    object,
                    footer,
                },
                size,
            ))
        } else {
            // We should not have leftover data
            Err(spa::pod::Error::Invalid)
        }
    }
}

impl Pod for Header {
    type DecodesTo = Self;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error> {
        if data.len() < 16 {
            return Err(spa::pod::Error::NoSpace);
        }

        data[0..4].copy_from_slice(&self.id.to_ne_bytes());
        let word = (self.opcode as u32) << 24 | (self.size & ((1 << 24) - 1));
        data[4..8].copy_from_slice(&word.to_ne_bytes());
        data[8..12].copy_from_slice(&self.seq.to_ne_bytes());
        data[12..16].copy_from_slice(&self.n_fds.to_ne_bytes());

        Ok(16)
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), spa::pod::Error> {
        if data.len() < 16 {
            return Err(spa::pod::Error::Invalid);
        }

        let id = u32::from_ne_bytes(data[0..4].try_into().unwrap());
        let word = u32::from_ne_bytes(data[4..8].try_into().unwrap());
        let opcode = (word >> 24) as u8;
        let size = word & ((1 << 24) - 1);
        let seq = u32::from_ne_bytes(data[8..12].try_into().unwrap());
        let n_fds = u32::from_ne_bytes(data[12..16].try_into().unwrap());

        Ok((
            Header {
                id,
                opcode,
                size,
                seq,
                n_fds,
            },
            16,
        ))
    }
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
