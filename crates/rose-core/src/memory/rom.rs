//! -----------------------------------------------------------------------
//! rom.rs: A wrapper over the N64 ROM data available from the cartridge
//!
//! A wrapper over ROM data with safe read accesses (mirrors addresses with
//! ROM size mask). The ROM data length must be a power of 2.
//!
//! Author(s): MrBubblezsz
//! -----------------------------------------------------------------------

use crate::memory::bus::MemoryAccess;

pub struct Rom {
    mask: usize,
    pub data: Vec<u8>,
}

impl Rom {
    pub fn new(data: Vec<u8>) -> anyhow::Result<Rom> {
        if !data.len().is_power_of_two() {
            return Err(anyhow::anyhow!("ROM data size must be a power of 2"));
        }

        Ok(Rom {
            mask: data.len() - 1,
            data,
        })
    }    
}

impl MemoryAccess for Rom {
    #[inline]
    fn read8(&self, paddr: u32) -> u8 {
        let addr = (paddr as usize) & self.mask;
        self.data[addr]
    }

    #[inline]
    fn read16(&self, paddr: u32) -> u16 {
        let addr = (paddr as usize) & self.mask;
        u16::from_be_bytes(self.data[addr..addr + 2].try_into().unwrap())
    }

    #[inline]
    fn read32(&self, paddr: u32) -> u32 {
        let addr = (paddr as usize) & self.mask;
        u32::from_be_bytes(self.data[addr..addr + 4].try_into().unwrap())
    }

    fn write8(&mut self, _addr: u32, _value: u8) { /* ROM data is not writable */ }
    fn write16(&mut self, _addr: u32, _value: u16) { /* ROM data is not writable */ }
    fn write32(&mut self, _addr: u32, _value: u32) { /* ROM data is not writable */ }
}