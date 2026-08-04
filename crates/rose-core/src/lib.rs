//! -----------------------------------------------------------------------
//! rose-core: N64 emulator core.
//!
//! Contains the Rose64Core struct that handles stepping the emulator through
//! frames.
//!
//! Author(s): MrBubblezsz
//! -----------------------------------------------------------------------

use crate::{memory::bus::Bus, processors::vr4300::CpuVR4300};

pub mod common;
pub mod memory;
pub mod processors;

pub struct Rose64Core {
    cpu: CpuVR4300,
    // rsp: Rsp,
    // rdp: Rdp,
    bus: Bus,
}

impl Rose64Core {
    pub fn new() -> Rose64Core {
        Rose64Core {
            cpu: CpuVR4300::new(),
            bus: Bus::new(vec![0u8; 0x1000]).ok().unwrap(), // temp blank rom data
        }
    }
}
