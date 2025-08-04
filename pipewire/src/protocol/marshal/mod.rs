// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

pub(crate) mod core;

use pipewire_native_macros as macros;
use pipewire_native_spa::{self as spa, pod::Pod};

pub(crate) trait Marshallable {
    fn opcode(&self) -> u8;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error>;
    fn decode(opcode: u8, data: &[u8]) -> Result<(Self, usize), spa::pod::Error>
    where
        Self: Sized;
}

pub(crate) struct Message<T: Marshallable> {
    pub(crate) header: Header,
    pub(crate) object: T,
    pub(crate) footer: Option<Footer>,
}

pub(crate) struct Header {
    pub(crate) id: u32,
    pub(crate) opcode: u8,
    pub(crate) size: u32, // actually 24 bytes
    pub(crate) seq: u32,
    pub(crate) n_fds: u32,
}

#[derive(macros::PodStruct)]
pub(crate) struct Footer {}

impl<T: Marshallable> Pod for Message<T> {
    type DecodesTo = Self;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error> {
        // Header + at least one byte for data
        if data.len() < 17 {
            return Err(spa::pod::Error::NoSpace);
        }

        let payload_size = self.object.encode(&mut data[16..])?;
        let footer_size = if let Some(footer) = &self.footer {
            footer.encode(&mut data[16 + payload_size..])?
        } else {
            0
        };

        let size = payload_size + footer_size;
        let header = Header {
            size: size as u32,
            ..self.header
        };

        header.encode(data)?;

        Ok(16 + size)
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), spa::pod::Error> {
        if data.len() < 16 {
            return Err(spa::pod::Error::Invalid);
        }

        let (header, header_size) = Header::decode(data)?;
        let size = header.size as usize;
        let (object, payload_size) = T::decode(header.opcode, &data[header_size..])?;

        let (footer, footer_size) = if size > header_size + payload_size {
            let (f, s) = Footer::decode(&data[header_size + payload_size..size])?;
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
        let word = u32::from_ne_bytes(data[0..4].try_into().unwrap());
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
