//! -----------------------------------------------------------------------
//! rdram.rs: A wrapper over 8 MiB RDRAM memory
//!
//! Provides an abstraction over RDRAM memory accessible to the CPU and RSP.
//! We allocate the full 8 MiB size of RDRAM available with the extension pack.
//!
//! Author(s): MrBubblezsz
//! -----------------------------------------------------------------------

use crate::{common::consts::MB, memory::bus::MemoryAccess};

/// Simple wrapper around 8 MiB of ram. The `data` field is kept public for
/// debug and testing access.
pub struct Rdram {
    /// SAFETY: A raw pointer to RDRAM data will be handed to JIT functions.
    ///         This data field must never be moved/reallocated after it is
    ///         created.
    pub data: Box<[u8]>,
}

impl Rdram {
    /// Max RDRAM size (with extension pak)
    pub const SIZE: usize = 8 * MB;

    pub fn new() -> Rdram {
        Rdram {
            data: vec![0; Rdram::SIZE].into_boxed_slice(),
        }
    }
}

impl MemoryAccess for Rdram {
    #[inline]
    fn read8(&self, paddr: u32) -> u8 {
        let addr = (paddr as usize) & (Rdram::SIZE - 1);
        self.data[addr]
    }

    #[inline]
    fn read16(&self, paddr: u32) -> u16 {
        let addr = (paddr as usize) & (Rdram::SIZE - 1);
        u16::from_be_bytes(self.data[addr..addr + 2].try_into().unwrap())
    }

    #[inline]
    fn read32(&self, paddr: u32) -> u32 {
        let addr = (paddr as usize) & (Rdram::SIZE - 1);
        u32::from_be_bytes(self.data[addr..addr + 4].try_into().unwrap())
    }

    #[inline]
    fn write8(&mut self, paddr: u32, value: u8) {
        let addr = (paddr as usize) & (Rdram::SIZE - 1);
        self.data[addr] = value;
    }
    
    #[inline]
    fn write16(&mut self, paddr: u32, value: u16) {
        let addr = (paddr as usize) & (Rdram::SIZE - 1);
        self.data[addr..addr + 2].copy_from_slice(&value.to_be_bytes());
    }
    
    #[inline]
    fn write32(&mut self, paddr: u32, value: u32) {
        let addr = (paddr as usize) & (Rdram::SIZE - 1);
        self.data[addr..addr + 4].copy_from_slice(&value.to_be_bytes());
    }
}
