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

use crate::memory::{Rom, fastmem::FastMemory};

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
    // pub sp_dmem: Box<[u8]>,
    // pub sp_imem: Box<[u8]>,
    // pub cart: Cart,
    // pub mmio: Mmio,
    pub memory: FastMemory,
    pub rom: Rom,
}

impl Bus {
    pub fn new(rom_data: Vec<u8>) -> anyhow::Result<Bus> {
        Ok(Bus {
            // sp_dmem: todo!(),
            // sp_imem: todo!(),
            memory: FastMemory::new(),
            rom: Rom::new(rom_data)?,
        })
    }
}

impl MemoryAccess for Bus {
    fn read8(&self, paddr: u32) -> u8 {
        match paddr {
            0x00000000..=0x003FFFFF => self.memory.rdram.read8(paddr), /* RDRAM */
            0x00400000..=0x007FFFFF => self.memory.rdram.read8(paddr), /* RDRAM (Expansion Pak) */
            0x00800000..=0x03EFFFFF => 0,                       /* Unused */
            0x03F00000..=0x03FFFFFF => 0,                       /* TODO: RDRAM Registers */
            0x04000000..=0x04000FFF => 0,                       /* TODO: SP DMEM */
            0x04001000..=0x04001FFF => 0,                       /* TODO: SP IMEM */
            0x04002000..=0x0403FFFF => 0,                       /* TODO: Unused */
            0x04040000..=0x040FFFFF => 0,                       /* TODO: SP Registers */
            0x04100000..=0x041FFFFF => 0,                       /* TODO: DP Command Registers */
            0x04200000..=0x042FFFFF => 0,                       /* TODO: DP Span Registers */
            0x04300000..=0x043FFFFF => 0,                       /* TODO: MIPS Interface (MI) */
            0x04400000..=0x044FFFFF => 0,                       /* TODO: Video Interface (VI) */
            0x04500000..=0x045FFFFF => 0,                       /* TODO: Audio Interface (AI) */
            0x04600000..=0x046FFFFF => 0, /* TODO: Peripheral Interface (PI) */
            0x04700000..=0x047FFFFF => 0, /* TODO: RDRAM Interface (RI) */
            0x04800000..=0x048FFFFF => 0, /* TODO: Serial Interface (SI) */
            0x04900000..=0x04FFFFFF => 0, /* TODO: Unused */
            0x05000000..=0x05FFFFFF => 0, /* TODO: N64DD control registers (open bus or all 0xFF when not present) */
            0x06000000..=0x07FFFFFF => 0, /* TODO: N64DD IPL ROM (open bus or all 0xFF when not present) */
            0x08000000..=0x0FFFFFFF => 0, /* TODO: SRAM */
            0x10000000..=0x1FBFFFFF => self.rom.read8(paddr), /* ROM */
            0x1FC00000..=0x1FC007BF => 0, /* TODO: PIF Boot Rom */
            0x1FC007C0..=0x1FC007FF => 0, /* TODO: PIF RAM */
            0x1FC00800..=0x1FCFFFFF => 0, /* TODO: Reserved */
            0x1FD00000..=0x7FFFFFFF => 0, /* TODO: Cartridge Domain 1 Address 3 */
            0x80000000..=0xFFFFFFFF => 0, /* TODO: Unknown */
        }
    }

    fn read16(&self, paddr: u32) -> u16 {
        match paddr {
            0x00000000..=0x003FFFFF => self.memory.rdram.read16(paddr), /* RDRAM */
            0x00400000..=0x007FFFFF => self.memory.rdram.read16(paddr), /* RDRAM (Expansion Pak) */
            0x00800000..=0x03EFFFFF => 0,                        /* Unused */
            0x03F00000..=0x03FFFFFF => 0,                        /* TODO: RDRAM Registers */
            0x04000000..=0x04000FFF => 0,                        /* TODO: SP DMEM */
            0x04001000..=0x04001FFF => 0,                        /* TODO: SP IMEM */
            0x04002000..=0x0403FFFF => 0,                        /* TODO: Unused */
            0x04040000..=0x040FFFFF => 0,                        /* TODO: SP Registers */
            0x04100000..=0x041FFFFF => 0,                        /* TODO: DP Command Registers */
            0x04200000..=0x042FFFFF => 0,                        /* TODO: DP Span Registers */
            0x04300000..=0x043FFFFF => 0,                        /* TODO: MIPS Interface (MI) */
            0x04400000..=0x044FFFFF => 0,                        /* TODO: Video Interface (VI) */
            0x04500000..=0x045FFFFF => 0,                        /* TODO: Audio Interface (AI) */
            0x04600000..=0x046FFFFF => 0, /* TODO: Peripheral Interface (PI) */
            0x04700000..=0x047FFFFF => 0, /* TODO: RDRAM Interface (RI) */
            0x04800000..=0x048FFFFF => 0, /* TODO: Serial Interface (SI) */
            0x04900000..=0x04FFFFFF => 0, /* TODO: Unused */
            0x05000000..=0x05FFFFFF => 0, /* TODO: N64DD control registers (open bus or all 0xFF when not present) */
            0x06000000..=0x07FFFFFF => 0, /* TODO: N64DD IPL ROM (open bus or all 0xFF when not present) */
            0x08000000..=0x0FFFFFFF => 0, /* TODO: SRAM */
            0x10000000..=0x1FBFFFFF => self.rom.read16(paddr), /* ROM */
            0x1FC00000..=0x1FC007BF => 0, /* TODO: PIF Boot Rom */
            0x1FC007C0..=0x1FC007FF => 0, /* TODO: PIF RAM */
            0x1FC00800..=0x1FCFFFFF => 0, /* TODO: Reserved */
            0x1FD00000..=0x7FFFFFFF => 0, /* TODO: Cartridge Domain 1 Address 3 */
            0x80000000..=0xFFFFFFFF => 0, /* TODO: Unknown */
        }
    }

    fn read32(&self, paddr: u32) -> u32 {
        match paddr {
            0x00000000..=0x003FFFFF => self.memory.rdram.read32(paddr), /* RDRAM */
            0x00400000..=0x007FFFFF => self.memory.rdram.read32(paddr), /* RDRAM (Expansion Pak) */
            0x00800000..=0x03EFFFFF => 0,                        /* Unused */
            0x03F00000..=0x03FFFFFF => 0,                        /* TODO: RDRAM Registers */
            0x04000000..=0x04000FFF => 0,                        /* TODO: SP DMEM */
            0x04001000..=0x04001FFF => 0,                        /* TODO: SP IMEM */
            0x04002000..=0x0403FFFF => 0,                        /* TODO: Unused */
            0x04040000..=0x040FFFFF => 0,                        /* TODO: SP Registers */
            0x04100000..=0x041FFFFF => 0,                        /* TODO: DP Command Registers */
            0x04200000..=0x042FFFFF => 0,                        /* TODO: DP Span Registers */
            0x04300000..=0x043FFFFF => 0,                        /* TODO: MIPS Interface (MI) */
            0x04400000..=0x044FFFFF => 0,                        /* TODO: Video Interface (VI) */
            0x04500000..=0x045FFFFF => 0,                        /* TODO: Audio Interface (AI) */
            0x04600000..=0x046FFFFF => 0, /* TODO: Peripheral Interface (PI) */
            0x04700000..=0x047FFFFF => 0, /* TODO: RDRAM Interface (RI) */
            0x04800000..=0x048FFFFF => 0, /* TODO: Serial Interface (SI) */
            0x04900000..=0x04FFFFFF => 0, /* TODO: Unused */
            0x05000000..=0x05FFFFFF => 0, /* TODO: N64DD control registers (open bus or all 0xFF when not present) */
            0x06000000..=0x07FFFFFF => 0, /* TODO: N64DD IPL ROM (open bus or all 0xFF when not present) */
            0x08000000..=0x0FFFFFFF => 0, /* TODO: SRAM */
            0x10000000..=0x1FBFFFFF => self.rom.read32(paddr), /* ROM */
            0x1FC00000..=0x1FC007BF => 0, /* TODO: PIF Boot Rom */
            0x1FC007C0..=0x1FC007FF => 0, /* TODO: PIF RAM */
            0x1FC00800..=0x1FCFFFFF => 0, /* TODO: Reserved */
            0x1FD00000..=0x7FFFFFFF => 0, /* TODO: Cartridge Domain 1 Address 3 */
            0x80000000..=0xFFFFFFFF => 0, /* TODO: Unknown */
        }
    }

    fn write8(&mut self, paddr: u32, value: u8) {
        match paddr {
            0x00000000..=0x003FFFFF => self.memory.rdram.write8(paddr, value), /* RDRAM */
            0x00400000..=0x007FFFFF => self.memory.rdram.write8(paddr, value), /* RDRAM (Expansion Pak) */
            0x00800000..=0x03EFFFFF => {}                               /* Unused */
            0x03F00000..=0x03FFFFFF => {}                               /* TODO: RDRAM Registers */
            0x04000000..=0x04000FFF => {}                               /* TODO: SP DMEM */
            0x04001000..=0x04001FFF => {}                               /* TODO: SP IMEM */
            0x04002000..=0x0403FFFF => {}                               /* TODO: Unused */
            0x04040000..=0x040FFFFF => {}                               /* TODO: SP Registers */
            0x04100000..=0x041FFFFF => {} /* TODO: DP Command Registers */
            0x04200000..=0x042FFFFF => {} /* TODO: DP Span Registers */
            0x04300000..=0x043FFFFF => {} /* TODO: MIPS Interface (MI) */
            0x04400000..=0x044FFFFF => {} /* TODO: Video Interface (VI) */
            0x04500000..=0x045FFFFF => {} /* TODO: Audio Interface (AI) */
            0x04600000..=0x046FFFFF => {} /* TODO: Peripheral Interface (PI) */
            0x04700000..=0x047FFFFF => {} /* TODO: RDRAM Interface (RI) */
            0x04800000..=0x048FFFFF => {} /* TODO: Serial Interface (SI) */
            0x04900000..=0x04FFFFFF => {} /* TODO: Unused */
            0x05000000..=0x05FFFFFF => {} /* TODO: N64DD control registers (open bus or all 0xFF when not present) */
            0x06000000..=0x07FFFFFF => {} /* TODO: N64DD IPL ROM (open bus or all 0xFF when not present) */
            0x08000000..=0x0FFFFFFF => {} /* TODO: SRAM */
            0x10000000..=0x1FBFFFFF => self.rom.write8(paddr, value), /* ROM */
            0x1FC00000..=0x1FC007BF => {} /* TODO: PIF Boot Rom */
            0x1FC007C0..=0x1FC007FF => {} /* TODO: PIF RAM */
            0x1FC00800..=0x1FCFFFFF => {} /* TODO: Reserved */
            0x1FD00000..=0x7FFFFFFF => {} /* TODO: Cartridge Domain 1 Address 3 */
            0x80000000..=0xFFFFFFFF => {} /* TODO: Unknown */
        }
    }

    fn write16(&mut self, paddr: u32, value: u16) {
        match paddr {
            0x00000000..=0x003FFFFF => self.memory.rdram.write16(paddr, value), /* RDRAM */
            0x00400000..=0x007FFFFF => self.memory.rdram.write16(paddr, value), /* RDRAM (Expansion Pak) */
            0x00800000..=0x03EFFFFF => {}                                /* Unused */
            0x03F00000..=0x03FFFFFF => {}                                /* TODO: RDRAM Registers */
            0x04000000..=0x04000FFF => {}                                /* TODO: SP DMEM */
            0x04001000..=0x04001FFF => {}                                /* TODO: SP IMEM */
            0x04002000..=0x0403FFFF => {}                                /* TODO: Unused */
            0x04040000..=0x040FFFFF => {}                                /* TODO: SP Registers */
            0x04100000..=0x041FFFFF => {} /* TODO: DP Command Registers */
            0x04200000..=0x042FFFFF => {} /* TODO: DP Span Registers */
            0x04300000..=0x043FFFFF => {} /* TODO: MIPS Interface (MI) */
            0x04400000..=0x044FFFFF => {} /* TODO: Video Interface (VI) */
            0x04500000..=0x045FFFFF => {} /* TODO: Audio Interface (AI) */
            0x04600000..=0x046FFFFF => {} /* TODO: Peripheral Interface (PI) */
            0x04700000..=0x047FFFFF => {} /* TODO: RDRAM Interface (RI) */
            0x04800000..=0x048FFFFF => {} /* TODO: Serial Interface (SI) */
            0x04900000..=0x04FFFFFF => {} /* TODO: Unused */
            0x05000000..=0x05FFFFFF => {} /* TODO: N64DD control registers (open bus or all 0xFF when not present) */
            0x06000000..=0x07FFFFFF => {} /* TODO: N64DD IPL ROM (open bus or all 0xFF when not present) */
            0x08000000..=0x0FFFFFFF => {} /* TODO: SRAM */
            0x10000000..=0x1FBFFFFF => self.rom.write16(paddr, value), /* ROM */
            0x1FC00000..=0x1FC007BF => {} /* TODO: PIF Boot Rom */
            0x1FC007C0..=0x1FC007FF => {} /* TODO: PIF RAM */
            0x1FC00800..=0x1FCFFFFF => {} /* TODO: Reserved */
            0x1FD00000..=0x7FFFFFFF => {} /* TODO: Cartridge Domain 1 Address 3 */
            0x80000000..=0xFFFFFFFF => {} /* TODO: Unknown */
        }
    }

    fn write32(&mut self, paddr: u32, value: u32) {
        match paddr {
            0x00000000..=0x003FFFFF => self.memory.rdram.write32(paddr, value), /* RDRAM */
            0x00400000..=0x007FFFFF => self.memory.rdram.write32(paddr, value), /* RDRAM (Expansion Pak) */
            0x00800000..=0x03EFFFFF => {}                                /* Unused */
            0x03F00000..=0x03FFFFFF => {}                                /* TODO: RDRAM Registers */
            0x04000000..=0x04000FFF => {}                                /* TODO: SP DMEM */
            0x04001000..=0x04001FFF => {}                                /* TODO: SP IMEM */
            0x04002000..=0x0403FFFF => {}                                /* TODO: Unused */
            0x04040000..=0x040FFFFF => {}                                /* TODO: SP Registers */
            0x04100000..=0x041FFFFF => {} /* TODO: DP Command Registers */
            0x04200000..=0x042FFFFF => {} /* TODO: DP Span Registers */
            0x04300000..=0x043FFFFF => {} /* TODO: MIPS Interface (MI) */
            0x04400000..=0x044FFFFF => {} /* TODO: Video Interface (VI) */
            0x04500000..=0x045FFFFF => {} /* TODO: Audio Interface (AI) */
            0x04600000..=0x046FFFFF => {} /* TODO: Peripheral Interface (PI) */
            0x04700000..=0x047FFFFF => {} /* TODO: RDRAM Interface (RI) */
            0x04800000..=0x048FFFFF => {} /* TODO: Serial Interface (SI) */
            0x04900000..=0x04FFFFFF => {} /* TODO: Unused */
            0x05000000..=0x05FFFFFF => {} /* TODO: N64DD control registers (open bus or all 0xFF when not present) */
            0x06000000..=0x07FFFFFF => {} /* TODO: N64DD IPL ROM (open bus or all 0xFF when not present) */
            0x08000000..=0x0FFFFFFF => {} /* TODO: SRAM */
            0x10000000..=0x1FBFFFFF => self.rom.write32(paddr, value), /* ROM */
            0x1FC00000..=0x1FC007BF => {} /* TODO: PIF Boot Rom */
            0x1FC007C0..=0x1FC007FF => {} /* TODO: PIF RAM */
            0x1FC00800..=0x1FCFFFFF => {} /* TODO: Reserved */
            0x1FD00000..=0x7FFFFFFF => {} /* TODO: Cartridge Domain 1 Address 3 */
            0x80000000..=0xFFFFFFFF => {} /* TODO: Unknown */
        }
    }
}
