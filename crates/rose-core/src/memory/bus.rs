//! -----------------------------------------------------------------------
//! bus.rs: Provides Bus struct and MemoryAccess trait
//!
//! Contains the Bus struct implementation, which owns various shared memories
//! and registers of the N64 and is responsible for mapping memory reads/writes
//! to their destination. Also provides the MemoryAccess trait, which mostly
//! exists to ensure that components follow a standard implementation pattern
//! for memory accesses.
//!
//! Author(s): MrBubblezsz
//! -----------------------------------------------------------------------

use crate::memory::{Rdram, Rom};

/// `MemoryAccess` contains methods for reading and writing various sizes of
/// data to and from underlying data structs. We may also leverage this for
/// debugging and testing by letting the CPU accept any bus that implements
/// `MemoryAccess`.
pub trait MemoryAccess {
    fn read8(&self, addr: u32) -> u8;
    fn read16(&self, addr: u32) -> u16;
    fn read32(&self, addr: u32) -> u32;

    fn write8(&mut self, addr: u32, value: u8);
    fn write16(&mut self, addr: u32, value: u16);
    fn write32(&mut self, addr: u32, value: u32);
}

/// The `Bus` struct contains resources that the CPU needs to be able to access
/// during its execution. It provides raw access for debug purposes, but the
/// CPU will only access the underlying data via the `MemoryAccess` trait.
pub struct Bus {
    pub rdram: Rdram,
    // pub sp_dmem: Box<[u8]>,
    // pub sp_imem: Box<[u8]>,
    // pub cart: Cart,
    // pub mmio: Mmio,
    pub rom: Rom,
}

impl Bus {
    pub fn new(rom_data: Vec<u8>) -> anyhow::Result<Bus> {
        Ok(Bus {
            rdram: Rdram::new(),
            // sp_dmem: todo!(),
            // sp_imem: todo!(),
            rom: Rom::new(rom_data)?,
        })
    }
}

impl MemoryAccess for Bus {
    fn read8(&self, paddr: u32) -> u8 {
        match paddr {
            0x0000_0000..=0x03FF_FFFF => self.rdram.read8(paddr),
            // 0x0400_0000..=0x040F_FFFF => self.sp_regs.read8(paddr),
            // 0x0410_0000..=0x041F_FFFF => self.dp_regs.read8(paddr),
            // 0x0430_0000..=0x043F_FFFF => self.mi_regs.read8(paddr),
            // 0x0440_0000..=0x044F_FFFF => self.vi_regs.read8(paddr),
            // 0x0450_0000..=0x045F_FFFF => self.ai_regs.read8(paddr),
            // 0x0460_0000..=0x046F_FFFF => self.pi_regs.read8(paddr),
            0x1000_0000..=0x1FBF_FFFF => self.rom.read8(paddr),
            // 0x1FC0_0000..=0x1FC0_07BF => self.pif.read8(paddr),
            _ => {
                /* open bus behavior */
                0
            }
        }
    }

    fn read16(&self, paddr: u32) -> u16 {
        match paddr {
            0x0000_0000..=0x03FF_FFFF => self.rdram.read16(paddr),
            // 0x0400_0000..=0x040F_FFFF => self.sp_regs.read16(paddr),
            // 0x0410_0000..=0x041F_FFFF => self.dp_regs.read16(paddr),
            // 0x0430_0000..=0x043F_FFFF => self.mi_regs.read16(paddr),
            // 0x0440_0000..=0x044F_FFFF => self.vi_regs.read16(paddr),
            // 0x0450_0000..=0x045F_FFFF => self.ai_regs.read16(paddr),
            // 0x0460_0000..=0x046F_FFFF => self.pi_regs.read16(paddr),
            0x1000_0000..=0x1FBF_FFFF => self.rom.read16(paddr),
            // 0x1FC0_0000..=0x1FC0_07BF => self.pif.read16(paddr),
            _ => {
                /* open bus behavior */
                0
            }
        }
    }

    fn read32(&self, paddr: u32) -> u32 {
        match paddr {
            0x0000_0000..=0x03FF_FFFF => self.rdram.read32(paddr),
            // 0x0400_0000..=0x040F_FFFF => self.sp_regs.read32(paddr),
            // 0x0410_0000..=0x041F_FFFF => self.dp_regs.read32(paddr),
            // 0x0430_0000..=0x043F_FFFF => self.mi_regs.read32(paddr),
            // 0x0440_0000..=0x044F_FFFF => self.vi_regs.read32(paddr),
            // 0x0450_0000..=0x045F_FFFF => self.ai_regs.read32(paddr),
            // 0x0460_0000..=0x046F_FFFF => self.pi_regs.read32(paddr),
            0x1000_0000..=0x1FBF_FFFF => self.rom.read32(paddr),
            // 0x1FC0_0000..=0x1FC0_07BF => self.pif.read32(paddr),
            _ => {
                /* open bus behavior */
                0
            }
        }
    }

    fn write8(&mut self, paddr: u32, value: u8) {
        match paddr {
            0x0000_0000..=0x03FF_FFFF => self.rdram.write8(paddr, value),
            // 0x0400_0000..=0x040F_FFFF => self.sp_regs.write8(paddr, value),
            // 0x0410_0000..=0x041F_FFFF => self.dp_regs.write8(paddr, value),
            // 0x0430_0000..=0x043F_FFFF => self.mi_regs.write8(paddr, value),
            // 0x0440_0000..=0x044F_FFFF => self.vi_regs.write8(paddr, value),
            // 0x0450_0000..=0x045F_FFFF => self.ai_regs.write8(paddr, value),
            // 0x0460_0000..=0x046F_FFFF => self.pi_regs.write8(paddr, value),
            0x1000_0000..=0x1FBF_FFFF => {} // Write to ROM
            // 0x1FC0_0000..=0x1FC0_07BF => self.pif.write8(paddr, value),
            _ => { /* open bus behavior */ }
        }
    }

    fn write16(&mut self, paddr: u32, value: u16) {
        match paddr {
            0x0000_0000..=0x03FF_FFFF => self.rdram.write16(paddr, value),
            // 0x0400_0000..=0x040F_FFFF => self.sp_regs.write16(paddr, value),
            // 0x0410_0000..=0x041F_FFFF => self.dp_regs.write16(paddr, value),
            // 0x0430_0000..=0x043F_FFFF => self.mi_regs.write16(paddr, value),
            // 0x0440_0000..=0x044F_FFFF => self.vi_regs.write16(paddr, value),
            // 0x0450_0000..=0x045F_FFFF => self.ai_regs.write16(paddr, value),
            // 0x0460_0000..=0x046F_FFFF => self.pi_regs.write16(paddr, value),
            0x1000_0000..=0x1FBF_FFFF => {} // Write to ROM
            // 0x1FC0_0000..=0x1FC0_07BF => self.pif.write16(paddr, value),
            _ => { /* open bus behavior */ }
        }
    }

    fn write32(&mut self, paddr: u32, value: u32) {
        match paddr {
            0x0000_0000..=0x03FF_FFFF => self.rdram.write32(paddr, value),
            // 0x0400_0000..=0x040F_FFFF => self.sp_regs.write32(paddr, value),
            // 0x0410_0000..=0x041F_FFFF => self.dp_regs.write32(paddr, value),
            // 0x0430_0000..=0x043F_FFFF => self.mi_regs.write32(paddr, value),
            // 0x0440_0000..=0x044F_FFFF => self.vi_regs.write32(paddr, value),
            // 0x0450_0000..=0x045F_FFFF => self.ai_regs.write32(paddr, value),
            // 0x0460_0000..=0x046F_FFFF => self.pi_regs.write32(paddr, value),
            0x1000_0000..=0x1FBF_FFFF => {} // Write to ROM
            // 0x1FC0_0000..=0x1FC0_07BF => self.pif.write32(paddr, value),
            _ => { /* open bus behavior */ }
        }
    }
}
