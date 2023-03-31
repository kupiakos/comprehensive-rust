// Copyright 2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use bitflags::bitflags;
use core::ptr::{addr_of, addr_of_mut};
use log::info;

macro_rules! read_sysreg {
    ($name:ident) => {
        {
            let mut value: u64;
            ::core::arch::asm!(
                concat!("mrs {value:x}, ", ::core::stringify!($name)),
                value = out(reg) value,
                options(nomem, nostack),
            );
            value
        }
    }
}

macro_rules! write_sysreg {
    ($name:ident, $value:expr) => {
        {
            let v: u64 = $value;
            ::core::arch::asm!(
                concat!("msr ", ::core::stringify!($name), ", {value:x}"),
                value = in(reg) v,
                options(nomem, nostack),
            )
        }
    }
}

/// The offset in bytes from `RD_base` to `SGI_base`.
const SGI_OFFSET: usize = 0x10000;

#[repr(C, align(8))]
struct GICD {
    /// Distributor control register.
    ctlr: u32,
    /// Interrupt controller type register.
    typer: u32,
    /// Distributor implementer identification register.
    iidr: u32,
    /// Interrupt controller type register 2.
    typer2: u32,
    /// Error reporting status register.
    statusr: u32,
    _reserved0: [u32; 3],
    /// Implementation defined registers.
    implementation_defined: [u32; 8],
    /// Set SPI register.
    setspi_nsr: u32,
    _reserved1: u32,
    /// Clear SPI register.
    clrspi_nsr: u32,
    _reserved2: u32,
    /// Set SPI secure register.
    setspi_sr: u32,
    _reserved3: u32,
    /// Clear SPI secure register.
    clrspi_sr: u32,
    _reserved4: [u32; 9],
    /// Interrupt group registers.
    igroupr: [u32; 32],
    /// Interrupt set-enable registers.
    isenabler: [u32; 32],
    /// Interrupt clear-enable registers.
    icenabler: [u32; 32],
    /// Interrupt set-pending registers.
    ispendr: [u32; 32],
    /// Interrupt clear-pending registers.
    icpendr: [u32; 32],
    /// Interrupt set-active registers.
    isactiver: [u32; 32],
    /// Interrupt clear-active registers.
    icactiver: [u32; 32],
    /// Interrupt priority registers.
    ipriorityr: [u8; 1024],
    /// Interrupt processor targets registers.
    itargetsr: [u32; 256],
    /// Interrupt configuration registers.
    icfgr: [u32; 64],
    /// Interrupt group modifier registers.
    igrpmodr: [u32; 32],
    _reserved5: [u32; 32],
    /// Non-secure access control registers.
    nsacr: [u32; 64],
    /// Software generated interrupt register.
    sigr: u32,
    _reserved6: [u32; 3],
    /// SGI clear-pending registers.
    cpendsgir: [u32; 4],
    /// SGI set-pending registers.
    spendsgir: [u32; 4],
    _reserved7: [u32; 20],
    /// Non-maskable interrupt registers.
    inmir: [u32; 32],
    /// Interrupt group registers for extended SPI range.
    igroupr_e: [u32; 32],
    _reserved8: [u32; 96],
    /// Interrupt set-enable registers for extended SPI range.
    isenabler_e: [u32; 32],
    _reserved9: [u32; 96],
    /// Interrupt clear-enable registers for extended SPI range.
    icenabler_e: [u32; 32],
    _reserved10: [u32; 96],
    /// Interrupt set-pending registers for extended SPI range.
    ispendr_e: [u32; 32],
    _reserved11: [u32; 96],
    /// Interrupt clear-pending registers for extended SPI range.
    icpendr_e: [u32; 32],
    _reserved12: [u32; 96],
    /// Interrupt set-active registers for extended SPI range.
    isactive_e: [u32; 32],
    _reserved13: [u32; 96],
    /// Interrupt clear-active registers for extended SPI range.
    icactive_e: [u32; 32],
    _reserved14: [u32; 224],
    /// Interrupt priority registers for extended SPI range.
    ipriorityr_e: [u8; 1024],
    _reserved15: [u32; 768],
    /// Extended SPI configuration registers.
    icfgr_e: [u32; 64],
    _reserved16: [u32; 192],
    /// Interrupt group modifier registers for extended SPI range.
    igrpmodr_e: [u32; 32],
    _reserved17: [u32; 96],
    /// Non-secure access control registers for extended SPI range.
    nsacr_e: [u32; 32],
    _reserved18: [u32; 288],
    /// Non-maskable interrupt registers for extended SPI range.
    inmr_e: [u32; 32],
    _reserved19: [u32; 2400],
    /// Interrupt routing registers.
    irouter: [u32; 1975],
    _reserved20: [u32; 9],
    /// Interrupt routing registers for extended SPI range.
    irouter_e: [u32; 2048],
    _reserved21: [u32; 2048],
    /// Implementation defined registers.
    implementation_defined2: [u32; 4084],
    /// ID registers.
    id_registers: [u32; 12],
}

bitflags! {
    #[repr(transparent)]
    #[derive(Copy, Clone, Debug, Eq, PartialEq)]
    struct Waker: u32 {
        const CHILDREN_ASLEEP = 1 << 2;
        const PROCESSOR_SLEEP = 1 << 1;
    }
}

#[repr(C, align(8))]
struct GICR {
    /// Redistributor control register.
    ctlr: u32,
    /// Implementer identification register.
    iidr: u32,
    /// Redistributor type register.
    typer: u64,
    /// Error reporting status register.
    statusr: u32,
    /// Redistributor wake register.
    waker: Waker,
    /// Report maximum PARTID and PMG register.
    mpamidr: u32,
    /// Set PARTID and PMG register.
    partidr: u32,
    /// Implementation defined registers.
    implementation_defined1: [u32; 8],
    /// Set LPI pending register.
    setlprir: u64,
    /// Clear LPI pending register.
    clrlpir: u64,
    _reserved0: [u32; 8],
    /// Redistributor properties base address register.
    propbaser: u64,
    /// Redistributor LPI pending table base address register.
    pendbaser: u64,
    _reserved1: [u32; 8],
    /// Redistributor invalidate LPI register.
    invlpir: u64,
    _reserved2: u64,
    /// Redistributor invalidate all register.
    invallr: u64,
    _reserved3: u64,
    /// Redistributor synchronize register.
    syncr: u32,
    _reserved4: [u32; 15],
    /// Implementation defined registers.
    implementation_defined2: u64,
    _reserved5: u64,
    /// Implementation defined registers.
    implementation_defined3: u64,
    _reserved6: [u32; 12218],
    /// Implementation defined registers.
    implementation_defined4: [u32; 4084],
    /// ID registers.
    id_registers: [u32; 12],
}

#[repr(C, align(8))]
struct SGI {
    _reserved0: [u32; 32],
    /// Interrupt group register 0.
    igroupr0: u32,
    /// Interrupt group registers for extended PPI range.
    igroupr_e: [u32; 2],
    _reserved1: [u32; 29],
    /// Interrupt set-enable register 0.
    isenabler0: u32,
    /// Interrupt set-enable registers for extended PPI range.
    isenabler_e: [u32; 2],
    _reserved2: [u32; 29],
    /// Interrupt clear-enable register 0.
    icenabler0: u32,
    /// Interrupt clear-enable registers for extended PPI range.
    icenabler_e: [u32; 2],
    _reserved3: [u32; 29],
    /// Interrupt set-pending register 0.
    ispendr0: u32,
    /// Interrupt set-pending registers for extended PPI range.
    ispendr_e: [u32; 2],
    _reserved4: [u32; 29],
    /// Interrupt clear-pending register 0.
    icpendr0: u32,
    /// Interrupt clear-pending registers for extended PPI range.
    icpendr_e: [u32; 2],
    _reserved5: [u32; 29],
    /// Interrupt set-active register 0.
    isactiver0: u32,
    /// Interrupt set-active registers for extended PPI range.
    isactive_e: [u32; 2],
    _reserved6: [u32; 29],
    /// Interrupt clear-active register 0.
    icactiver0: u32,
    /// Interrupt clear-active registers for extended PPI range.
    icactive_e: [u32; 2],
    _reserved7: [u32; 29],
    /// Interrupt priority registers.
    ipriorityr: [u8; 32],
    /// Interrupt priority registers for extended PPI range.
    ipriorityr_e: [u8; 64],
    _reserved8: [u32; 488],
    /// SGI configuration register.
    icfgr0: u32,
    /// PPI configuration register.
    icfgr1: u32,
    /// Extended PPI configuration registers.
    icfgr_e: [u32; 4],
    _reserved9: [u32; 58],
    /// Interrupt group modifier register 0.
    igrpmodr0: u32,
    /// Interrupt group modifier registers for extended PPI range.
    igrpmodr_e: [u32; 2],
    _reserved10: [u32; 61],
    /// Non-secure access control register.
    nsacr: u32,
    _reserved11: [u32; 95],
    /// Non-maskable interrupt register for PPIs.
    inmir0: u32,
    /// Non-maskable interrupt register for extended PPIs.
    inmir_e: [u32; 31],
    _reserved12: [u32; 11264],
    /// Implementation defined registers.
    implementation_defined: [u32; 4084],
    _reserved13: [u32; 12],
}

#[derive(Debug)]
pub struct GicV3 {
    gicd: *mut GICD,
    gicr: *mut GICR,
    sgi: *mut SGI,
}

impl GicV3 {
    pub unsafe fn new(gicd: *mut u64, gicr: *mut u64) -> Self {
        Self {
            gicd: gicd as _,
            gicr: gicr as _,
            sgi: gicr.offset(SGI_OFFSET) as _,
        }
    }

    pub fn setup(&mut self) {
        unsafe {
            info!("ICC_CTLR_EL1={:#x}", read_sysreg!(icc_ctlr_el1));
            write_sysreg!(icc_ctlr_el1, 0);
            info!("ICC_CTLR_EL1={:#x}", read_sysreg!(icc_ctlr_el1));
        }

        // Enable affinity routing and group 1 non-secure interrupts.
        let ctlr = 0x1 << 4 | 0x1 << 1;
        unsafe {
            addr_of_mut!((*self.gicd).ctlr).write_volatile(ctlr);
        }

        // Mark CPU as awake.
        unsafe {
            let mut waker = addr_of!((*self.gicr).waker).read_volatile();
            info!("WAKER: {:?}", waker);
            waker -= Waker::PROCESSOR_SLEEP;
            info!("Writing WAKER: {:?}", waker);
            addr_of_mut!((*self.gicr).waker).write_volatile(waker);

            info!("WAKER: {:?}", addr_of!((*self.gicr).waker).read_volatile());
        }

        // Put interrupts into non-secure group 1.
        unsafe {
            addr_of_mut!((*self.gicd).igroupr[0]).write_volatile(0xffffffff);
        }

        // Enable non-secure group 1.
        unsafe {
            write_sysreg!(icc_igrpen1_el1, 0x00000001);
        }
    }
    }
}
