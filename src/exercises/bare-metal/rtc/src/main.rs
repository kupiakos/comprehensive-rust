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

// ANCHOR: top
#![no_main]
#![no_std]

mod exceptions;
mod gicv3;
mod logger;
mod pl011;
// ANCHOR_END: top
mod pl031;

use crate::gicv3::{irq_enable, GicV3, Trigger};
use crate::pl031::Rtc;
use chrono::{TimeZone, Utc};
// ANCHOR: imports
use crate::pl011::Uart;
use core::{hint::spin_loop, panic::PanicInfo};
use log::{error, info, LevelFilter};
use psci::system_off;

/// Base addresses of the GICv3.
const GICD_BASE_ADDRESS: *mut u64 = 0x800_0000 as _;
const GICR_BASE_ADDRESS: *mut u64 = 0x80A_0000 as _;

/// Base address of the primary PL011 UART.
const PL011_BASE_ADDRESS: *mut u32 = 0x900_0000 as _;
// ANCHOR_END: imports

/// Base address of the PL031 RTC.
const PL031_BASE_ADDRESS: *mut u32 = 0x901_0000 as _;
// SPI interrupt 2, level triggered
const PL031_IRQ: u32 = 2;

// ANCHOR: main
#[no_mangle]
extern "C" fn main(x0: u64, x1: u64, x2: u64, x3: u64) {
    // Safe because `PL011_BASE_ADDRESS` is the base address of a PL011 device,
    // and nothing else accesses that address range.
    let uart = unsafe { Uart::new(PL011_BASE_ADDRESS) };
    logger::init(uart, LevelFilter::Trace).unwrap();

    info!("main({:#x}, {:#x}, {:#x}, {:#x})", x0, x1, x2, x3);
    // ANCHOR_END: main

    let mut gic = unsafe { GicV3::new(GICD_BASE_ADDRESS, GICR_BASE_ADDRESS) };
    gic.setup();

    // Test sending an SGI.
    let sgi_intid = 3;
    GicV3::set_priority_mask(0xff);
    gic.set_interrupt_priority(sgi_intid.into(), 0x80);
    irq_enable();
    gic.enable_all_interrupts(true);
    assert_eq!(gic.gicd_pending(0), 0);
    assert_eq!(gic.gicr_pending(), 0);
    assert_eq!(gic.gicd_active(0), 0);
    assert_eq!(gic.gicr_active(), 0);
    info!("Sending SGI");
    GicV3::send_sgi(sgi_intid, false, 0, 0, 0, 1);
    info!("Sent SGI");
    assert_eq!(gic.gicd_pending(0), 0);
    assert_eq!(gic.gicr_pending(), 0);
    assert_eq!(gic.gicd_active(0), 0);
    assert_eq!(gic.gicr_active(), 0);

    // Safe because `PL031_BASE_ADDRESS` is the base address of a PL031 device,
    // and nothing else accesses that address range.
    let mut rtc = unsafe { Rtc::new(PL031_BASE_ADDRESS) };
    let timestamp = rtc.read();
    let time = Utc.timestamp_opt(timestamp.into(), 0).unwrap();
    info!("RTC: {time}");

    GicV3::set_priority_mask(0xff);
    gic.set_interrupt_priority(PL031_IRQ, 0x80);
    gic.set_trigger(PL031_IRQ, Trigger::Level);
    irq_enable();
    gic.enable_all_interrupts(true);

    let target = timestamp + 3;
    rtc.set_match(target);
    rtc.mask_interrupt(false);
    info!(
        "Waiting for {}",
        Utc.timestamp_opt(target.into(), 0).unwrap()
    );
    while !rtc.matched() {
        spin_loop();
    }
    info!("Finished waiting");

    // ANCHOR: main_end
    system_off().unwrap();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("{info}");
    system_off().unwrap();
    loop {}
}
// ANCHOR_END: main_end
