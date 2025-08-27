// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Arun Raghavan

use pipewire_native_macros::EnumU32;

use crate::pod::types::ObjectType;

use super::ParamObject;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumU32)]
pub enum MediaType {
    Unknown,
    Audio,
    Video,
    Image,
    Binary,
    Stream,
    Application,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumU32)]
pub enum MediaSubtype {
    Unknown,
    Raw,
    Dsp,
    Iec958,
    Dsd,

    StartAudio = 0x10000,
    Mp3,
    Aac,
    Vorbis,
    Wma,
    Ra,
    Sbc,
    Adpcm,
    G723,
    G726,
    G729,
    Amr,
    Gsm,
    Alac,
    Flac,
    Ape,
    Opus,

    StartVideo = 0x20000,
    H264,
    Mjpg,
    Dv,
    Mpegts,
    H263,
    Mpeg1,
    Mpeg2,
    Mpeg4,
    Xvid,
    Vc1,
    Vp8,
    Vp9,
    Bayer,
    H265,

    StartImage = 0x30000,
    Jpeg,

    StartBinary = 0x40000,

    StartStream = 0x50000,
    Midi,

    StartApplication = 0x60000,
    Control,
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, EnumU32)]
pub enum Format {
    Start,

    MediaType,
    MediaSubtype,

    StartAudio = 0x10000,
    AudioFormat,
    AudioFlags,
    AudioRate,
    AudioChannels,
    AudioPosition,
    AudioIec958Codec,
    AudioBitorder,
    AudioInterleave,
    AudioBitrate,
    AudioBlockAlign,
    AudioAacStreamFormat,
    AudioWmaProfile,
    AudioAmrBandMode,

    StartVideo = 0x20000,
    VideoFormat,
    VideoModifier,
    VideoSize,
    VideoFramerate,
    VideoMaxFramerate,
    VideoViews,
    VideoInterlaceMode,
    VideoPixelAspectRatio,
    VideoMultiviewMode,
    VideoMultiviewFlags,
    VideoChromaSite,
    VideoColorRange,
    VideoColorMatrix,
    VideoTransferFunction,
    VideoColorPrimaries,
    VideoProfile,
    VideoLevel,
    VideoH264StreamFormat,
    VideoH264Alignment,
    VideoH265StreamFormat,
    VideoH265Alignment,

    StartImage = 0x30000,

    StartBinary = 0x40000,

    StartStream = 0x50000,

    StartApplication = 0x60000,
    ControlTypes,
}

impl ParamObject for Format {
    const TYPE: ObjectType = ObjectType::Format;
}
