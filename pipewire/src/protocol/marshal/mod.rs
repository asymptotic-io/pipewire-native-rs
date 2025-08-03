// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

pub(crate) mod core;

use pipewire_native_macros as macros;
use pipewire_native_spa::{self as spa, pod::Pod};

pub(crate) struct Message<O: Pod<DecodesTo = O>> {
    pub(crate) header: Header,
    pub(crate) object: O,
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

impl<O: Pod<DecodesTo = O>> Pod for Message<O> {
    type DecodesTo = Self;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error> {
        // Header + at least one byte for data
        if data.len() < 5 {
            return Err(spa::pod::Error::NoSpace);
        }

        let payload_size = self.object.encode(&mut data[4..])?;
        let footer_size = if let Some(footer) = &self.footer {
            footer.encode(&mut data[4 + payload_size..])?
        } else {
            0
        };

        let size = 4 + payload_size + footer_size;
        let header = Header {
            size: size as u32,
            ..self.header
        };

        header.encode(data)?;

        Ok(size)
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), spa::pod::Error> {
        if data.len() < 4 {
            return Err(spa::pod::Error::Invalid);
        }

        let (header, header_size) = Header::decode(data)?;
        let size = header.size as usize;
        let (object, payload_size) = O::decode(&data[header_size..])?;

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
        if data.len() < 4 {
            return Err(spa::pod::Error::NoSpace);
        }

        data[0..4].copy_from_slice(&self.id.to_ne_bytes());
        data[4] = self.opcode;
        data[5..8].copy_from_slice(&self.size.to_ne_bytes()[0..3]);
        data[8..12].copy_from_slice(&self.seq.to_ne_bytes());
        data[12..16].copy_from_slice(&self.n_fds.to_ne_bytes());

        Ok(16)
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), spa::pod::Error> {
        if data.len() < 4 {
            return Err(spa::pod::Error::Invalid);
        }

        let id = u32::from_ne_bytes(data[0..4].try_into().unwrap());
        let opcode = data[4];
        let size = (data[7] as u32) << 16 | (data[6] as u32) << 8 | data[5] as u32;
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
