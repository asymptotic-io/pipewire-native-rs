// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa as spa;

use super::{Marshallable, HEADER_LEN};

pub(crate) struct Message<T: Marshallable, F: spa::pod::Pod> {
    pub(crate) header: Header,
    pub(crate) object: T,
    pub(crate) footer: Option<F>,
}

pub(crate) struct Header {
    pub(crate) id: u32,
    pub(crate) opcode: u8,
    pub(crate) size: u32, // actually 24 bytes
    pub(crate) seq: u32,
    pub(crate) n_fds: u32,
}

impl<T: Marshallable, F: spa::pod::Pod<DecodesTo = F>> spa::pod::Pod for Message<T, F> {
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

impl spa::pod::Pod for Header {
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

pub(crate) struct CoreFooter {
    pub(crate) payloads: Vec<CoreFooterPayload>,
}

impl CoreFooter {
    pub(crate) fn new() -> Self {
        CoreFooter { payloads: vec![] }
    }

    #[allow(unused)]
    pub(crate) fn push(&mut self, payload: CoreFooterPayload) {
        self.payloads.push(payload);
    }
}

pub(crate) struct ClientFooter {
    pub(crate) payloads: Vec<ClientFooterPayload>,
}

impl ClientFooter {
    pub(crate) fn new() -> Self {
        ClientFooter { payloads: vec![] }
    }

    pub(crate) fn push(&mut self, payload: ClientFooterPayload) {
        self.payloads.push(payload);
    }
}
pub(crate) enum CoreFooterPayload {
    Generation(CoreGeneration),
}

pub(crate) enum ClientFooterPayload {
    Generation(ClientGeneration),
}

#[derive(macros::PodStruct)]
pub(crate) struct CoreGeneration {
    pub(crate) registry_generation: i64,
}

#[derive(macros::PodStruct)]
pub(crate) struct ClientGeneration {
    pub(crate) client_generation: i64,
}

impl spa::pod::Pod for CoreFooter {
    type DecodesTo = Self;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error> {
        let mut builder = spa::pod::builder::Builder::new(data);

        builder = builder.push_struct(|mut sb| {
            for p in &self.payloads {
                sb = match p {
                    CoreFooterPayload::Generation(g) => {
                        sb.push_id(spa::pod::types::Id(0u32)).push_pod(g)
                    }
                };
            }

            sb
        });

        let out = builder.build()?;

        Ok(out.len())
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), spa::pod::Error> {
        let mut parser = spa::pod::parser::Parser::new(data);

        parser.pop_struct(|sp| {
            let mut footer = CoreFooter::new();

            while sp.available() > 0 {
                let opcode = sp.pop_id::<u32>()?;
                let payload = match opcode.0 {
                    0 => {
                        let g = sp.pop_pod::<CoreGeneration>()?;
                        CoreFooterPayload::Generation(g)
                    }
                    _ => return Err(spa::pod::Error::Invalid),
                };

                footer.payloads.push(payload);
            }

            Ok(footer)
        })
    }
}

impl spa::pod::Pod for ClientFooter {
    type DecodesTo = Self;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error> {
        let mut builder = spa::pod::builder::Builder::new(data);

        builder = builder.push_struct(|mut sb| {
            for p in &self.payloads {
                sb = match p {
                    ClientFooterPayload::Generation(g) => {
                        sb.push_id(spa::pod::types::Id(0u32)).push_pod(g)
                    }
                };
            }

            sb
        });

        let out = builder.build()?;

        Ok(out.len())
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), spa::pod::Error> {
        let mut parser = spa::pod::parser::Parser::new(data);

        parser.pop_struct(|sp| {
            let mut footer = ClientFooter::new();

            while sp.available() > 0 {
                let opcode = sp.pop_id::<u32>()?;
                let payload = match opcode.0 {
                    0 => {
                        let g = sp.pop_pod::<ClientGeneration>()?;
                        ClientFooterPayload::Generation(g)
                    }
                    _ => return Err(spa::pod::Error::Invalid),
                };

                footer.payloads.push(payload);
            }

            Ok(footer)
        })
    }
}
