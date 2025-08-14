// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros as macros;
use pipewire_native_spa as spa;

use super::Marshallable;

pub(crate) struct Message<T: Marshallable, F: spa::pod::Pod> {
    pub(crate) header: Header,
    pub(crate) object: T,
    pub(crate) footer: F,
}

pub(crate) struct Header {
    pub(crate) id: u32,
    pub(crate) opcode: u8,
    pub(crate) size: u32, // actually 24 bytes
    pub(crate) seq: u32,
    pub(crate) n_fds: u32,
}

pub(crate) struct CoreFooter {
    payload: Vec<CoreFooterPayload>,
}

impl CoreFooter {
    pub(crate) fn new() -> Self {
        CoreFooter { payload: vec![] }
    }
}

pub(crate) struct ClientFooter {
    payload: Vec<ClientFooterPayload>,
}

impl ClientFooter {
    pub(crate) fn new() -> Self {
        ClientFooter { payload: vec![] }
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
    registry_generation: i64,
}

#[derive(macros::PodStruct)]
pub(crate) struct ClientGeneration {
    registry_generation: i64,
}

impl spa::pod::Pod for CoreFooter {
    type DecodesTo = Self;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error> {
        let mut builder = spa::pod::builder::Builder::new(data);

        for p in &self.payload {
            builder = match p {
                CoreFooterPayload::Generation(g) => {
                    builder
                        .push_int(0) // opcode for Core Generation
                        .push_pod(g)
                }
            };
        }

        let out = builder.build()?;

        Ok(out.len())
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), spa::pod::Error> {
        let mut footer = CoreFooter::new();
        let mut parser = spa::pod::parser::Parser::new(data);

        while parser.available() > 0 {
            let opcode = parser.pop_int()?;
            let payload = match opcode {
                0 => {
                    let g = parser.pop_pod::<CoreGeneration>()?;
                    CoreFooterPayload::Generation(g)
                }
                _ => return Err(spa::pod::Error::Invalid),
            };

            footer.payload.push(payload);
        }

        Ok((footer, data.len()))
    }
}

impl spa::pod::Pod for ClientFooter {
    type DecodesTo = Self;

    fn encode(&self, data: &mut [u8]) -> Result<usize, spa::pod::Error> {
        let mut builder = spa::pod::builder::Builder::new(data);

        for p in &self.payload {
            builder = match p {
                ClientFooterPayload::Generation(g) => {
                    builder
                        .push_int(0) // opcode for Client Generation
                        .push_pod(g)
                }
            };
        }

        let out = builder.build()?;

        Ok(out.len())
    }

    fn decode(data: &[u8]) -> Result<(Self::DecodesTo, usize), spa::pod::Error> {
        let mut footer = ClientFooter::new();
        let mut parser = spa::pod::parser::Parser::new(data);

        while parser.available() > 0 {
            let opcode = parser.pop_int()?;
            let payload = match opcode {
                0 => {
                    let g = parser.pop_pod::<ClientGeneration>()?;
                    ClientFooterPayload::Generation(g)
                }
                _ => return Err(spa::pod::Error::Invalid),
            };

            footer.payload.push(payload);
        }

        Ok((footer, data.len()))
    }
}
