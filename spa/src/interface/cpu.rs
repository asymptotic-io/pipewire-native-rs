// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 Asymptotic Inc.
// SPDX-FileCopyrightText: Copyright (c) 2025 Sanchayan Maity

use super::plugin::Interface;
use bitflags::bitflags;
use pipewire_native_macros::EnumU32;
use std::{any::Any, pin::Pin};

bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, EnumU32)]
    pub struct X86CpuFlags: u32 {
        const MMX            = (1<<0);	/* standard MMX */
        const MMXEXT         = (1<<1);	/* SSE integer or AMD MMX ext */
        const AMD_3DNOW      = (1<<2);	/* AMD 3DNOW */
        const SSE            = (1<<3);	/* SSE */
        const SSE2           = (1<<4);	/* SSE2 */
        const AMD_3DNOWEXT   = (1<<5);	/* AMD 3DNowExt */
        const SSE3           = (1<<6);	/* Prescott SSE3 */
        const SSSE3          = (1<<7);	/* Conroe SSSE3 */
        const SSE41          = (1<<8);	/* Penryn SSE4.1 */
        const SSE42          = (1<<9);	/* Nehalem SSE4.2 */
        const AESNI          = (1<<10);	/* Advanced Encryption Standard */
        const AVX            = (1<<11);	/* AVX */
        const XOP            = (1<<12);	/* Bulldozer XOP */
        const FMA4           = (1<<13);	/* Bulldozer FMA4 */
        const CMOV           = (1<<14);	/* supports cmov */
        const AVX2           = (1<<15);	/* AVX2 */
        const FMA3           = (1<<16);	/* Haswell FMA3 */
        const BMI1           = (1<<17);	/* Bit Manipulation Instruction Set 1 */
        const BMI2           = (1<<18);	/* Bit Manipulation Instruction Set 2 */
        const AVX512         = (1<<19);	/* AVX-512 */
        const SLOW_UNALIGNED = (1<<20);	/* unaligned loads/stores are slow */
        const _              = !0;      /* The source may set any bits */
    }
}

bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, EnumU32)]
    pub struct PPCCpuFlags: u32 {
        const ALTIVEC = (1<<0);	/* standard */
        const VSX     = (1<<1);	/* ISA 2.06 */
        const POWER8  = (1<<2);	/* ISA 2.07 */
        const _       = !0;     /* The source may set any bits */
    }
}

bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, EnumU32)]
    pub struct ARMCpuFlags : u32 {
        const ARMV5TE = (1 << 0);
        const ARMV6   = (1 << 1);
        const ARMV6T2 = (1 << 2);
        const VFP     = (1 << 3);
        const VFPV3   = (1 << 4);
        const NEON    = (1 << 5);
        const ARMV8   = (1 << 6);
        const _       = !0;       /* The source may set any bits */
    }
}

bitflags! {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, EnumU32)]
    pub struct RISCVCpuFlags : u32 {
        const RISCV_V = (1 << 0);
        const _       = !0;       /* The source may set any bits */
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumU32)]
pub enum CpuFlags {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    X86CpuFlags,
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    ARMCpuFlags,
    #[cfg(any(target_arch = "powerpc64", target_arch = "powerpc"))]
    PPCCpuFlags,
    #[cfg(target_arch = "riscv64")]
    RISCVCpuFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumU32)]
pub enum CpuVm {
    None = 0,
    Other,
    Kvm,
    Qemu,
    Bochs,
    Xen,
    Uml,
    Vmware,
    Oracle,
    Microsoft,
    Zvm,
    Parallels,
    Bhyve,
    Qnx,
    Acrn,
    PowerVm,
}

impl std::fmt::Display for CpuVm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let res = match self {
            CpuVm::None => "0",
            CpuVm::Other => "other",
            CpuVm::Kvm => "kvm",
            CpuVm::Qemu => "qemu",
            CpuVm::Bochs => "bochs",
            CpuVm::Xen => "xen",
            CpuVm::Uml => "uml",
            CpuVm::Vmware => "vmware",
            CpuVm::Oracle => "oracle",
            CpuVm::Microsoft => "microsoft",
            CpuVm::Zvm => "zvm",
            CpuVm::Parallels => "parallels",
            CpuVm::Bhyve => "bhyve",
            CpuVm::Qnx => "qnx",
            CpuVm::Acrn => "acrn",
            CpuVm::PowerVm => "powervm",
        };

        write!(f, "{}", res)
    }
}

pub struct CpuImpl {
    pub inner: Pin<Box<dyn Any>>,

    pub get_flags: fn(this: &CpuImpl) -> CpuFlags,
    pub force_flags: fn(this: &CpuImpl, flags: CpuFlags) -> i32,
    pub get_count: fn(this: &CpuImpl) -> u32,
    pub get_max_align: fn(this: &CpuImpl) -> u32,
    pub get_vm_type: fn(this: &CpuImpl) -> CpuVm,
    pub zero_denormals: fn(this: &CpuImpl, enable: bool) -> i32,
}

impl CpuImpl {
    pub fn get_flags(&self) -> CpuFlags {
        (self.get_flags)(self)
    }

    pub fn force_flags(&self, flags: CpuFlags) -> i32 {
        (self.force_flags)(self, flags)
    }

    pub fn get_count(&self) -> u32 {
        (self.get_count)(self)
    }

    pub fn get_max_align(&self) -> u32 {
        (self.get_max_align)(self)
    }

    pub fn get_vm_type(&self) -> CpuVm {
        (self.get_vm_type)(self)
    }

    pub fn zero_denormals(&self, enable: bool) -> i32 {
        (self.zero_denormals)(self, enable)
    }
}

impl Interface for CpuImpl {}

unsafe impl Send for CpuImpl {}
unsafe impl Sync for CpuImpl {}
