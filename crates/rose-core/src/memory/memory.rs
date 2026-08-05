//! -----------------------------------------------------------------------
//! memory.rs: An implementation of fast memory
//!
//! The FastMemory struct contains portions of the N64's internal memory that
//! are statically mapped to parts of virtal address space. This includes
//! RDRAM, iMem, and dMem, all accessible through sections KSEG0 and KSEG1.
//! The struct also contains a lookup table for pages of these memory segments.
//! The CPU uses the lookup table to skip vaddr->paddr translation for these
//! memory regions.
//!
//! Author(s): MrBubblezsz
//! -----------------------------------------------------------------------

use std::ptr::NonNull;

use crate::{
    common::consts::{GB, KB, MB},
    memory::bus::MemoryAccess,
};

pub const PMEM_SIZE: usize = 4 * GB;

const RDRAM_SIZE: usize = 8 * MB;
const IMEM_SIZE: usize = 4 * KB;
const DMEM_SIZE: usize = 4 * KB;

/// Simple wrapper around 8 MiB of ram. The internal data is kept public for
/// debug and testing access.
pub struct Rdram(pub Box<[u8]>);

/// Owns RDRAM, I-Mem, and D-Mem data. Provides fast CPU access via a table
/// lookup for 4 KiB virtual pages to skip virtual -> physical address
/// translation. As I-Mem and D-Mem are both 4 KiB in size, the pages cannot
/// be any larger than this.
pub struct FastMemory {
    pub rdram: Rdram,
    pub sp_imem: Box<[u8]>,
    pub sp_dmem: Box<[u8]>,

    /// ## SAFETY:
    ///   Raw pointers to `rdram`, `sp_imem`, and `sp_dmem` will be stored in
    ///   the `vmem_lookup` table. These memory arrays must not be reallocated
    ///   or moved after their creation.
    vmem_lookup: Box<[Option<NonNull<u8>>]>,
}

impl FastMemory {
    const VPAGE_BITS: usize = 12;
    const VPAGE_SIZE: usize = 1 << Self::VPAGE_BITS;
    const NUM_LOOKUP_ENTRIES: usize = PMEM_SIZE / Self::VPAGE_SIZE;

    pub fn new() -> FastMemory {
        let mut mem = FastMemory {
            rdram: Rdram(vec![0u8; RDRAM_SIZE].into_boxed_slice()),
            sp_imem: vec![0u8; IMEM_SIZE].into_boxed_slice(),
            sp_dmem: vec![0u8; DMEM_SIZE].into_boxed_slice(),
            vmem_lookup: vec![None; Self::NUM_LOOKUP_ENTRIES].into_boxed_slice(),
        };

        mem.init();

        mem
    }

    /// Zeros out RDRAM, Instruction Memory, and Data Memory
    pub fn clear_all(&mut self) {
        self.rdram.0.fill(0u8);
        self.sp_imem.fill(0u8);
        self.sp_dmem.fill(0u8);
    }

    /// Initialize the virtual address lookup table with raw pointers to pages
    /// of `rdram`, `sp_dmem`, and `sp_imem`. These fields cannot be moved or
    /// reallocated after this function has been called.
    fn init(&mut self) {
        const KSEG0_BASE: usize = 0x80000000;
        const KSEG1_BASE: usize = 0xA0000000;
        const RDRAM_PBASE: usize = 0x00000000;
        const DMEM_PBASE: usize = 0x04000000;
        const IMEM_PBASE: usize = 0x04001000;

        self.vmem_lookup.fill(None);

        let mut map_vmem = |pbase: usize, size: usize, mem: &mut [u8]| {
            assert_eq!(mem.len(), size);

            let num_pages = size / FastMemory::VPAGE_SIZE;

            for i in 0..num_pages {
                let offset = i * FastMemory::VPAGE_SIZE;
                let kseg0_vaddr = KSEG0_BASE + pbase + offset;
                let kseg1_vaddr = KSEG1_BASE + pbase + offset;

                let page_ptr = NonNull::new(&mut mem[offset]).unwrap();

                self.vmem_lookup[kseg0_vaddr >> 12] = Some(page_ptr);
                self.vmem_lookup[kseg1_vaddr >> 12] = Some(page_ptr);
            }
        };

        map_vmem(RDRAM_PBASE, RDRAM_SIZE, &mut self.rdram.0);
        map_vmem(DMEM_PBASE, DMEM_SIZE, &mut self.sp_dmem);
        map_vmem(IMEM_PBASE, IMEM_SIZE, &mut self.sp_imem);
    }

    /// Returns a raw pointer to the byte at `vaddr`, or None if unmapped.
    pub fn get_raw_mem(&mut self, vaddr: u32) -> Option<*mut u8> {
        let page = (vaddr as usize) >> Self::VPAGE_BITS;
        let off = (vaddr as usize) & (Self::VPAGE_SIZE - 1);
        self.vmem_lookup[page].map(|p| unsafe { p.as_ptr().add(off) })
    }
}

impl MemoryAccess for Rdram {
    #[inline]
    fn read8(&self, paddr: u32) -> u8 {
        let addr = (paddr as usize) & (RDRAM_SIZE - 1);
        self.0[addr]
    }

    #[inline]
    fn read16(&self, paddr: u32) -> u16 {
        let addr = (paddr as usize) & (RDRAM_SIZE - 1);
        u16::from_be_bytes(self.0[addr..addr + 2].try_into().unwrap())
    }

    #[inline]
    fn read32(&self, paddr: u32) -> u32 {
        let addr = (paddr as usize) & (RDRAM_SIZE - 1);
        u32::from_be_bytes(self.0[addr..addr + 4].try_into().unwrap())
    }

    #[inline]
    fn write8(&mut self, paddr: u32, value: u8) {
        let addr = (paddr as usize) & (RDRAM_SIZE - 1);
        self.0[addr] = value;
    }

    #[inline]
    fn write16(&mut self, paddr: u32, value: u16) {
        let addr = (paddr as usize) & (RDRAM_SIZE - 1);
        self.0[addr..addr + 2].copy_from_slice(&value.to_be_bytes());
    }

    #[inline]
    fn write32(&mut self, paddr: u32, value: u32) {
        let addr = (paddr as usize) & (RDRAM_SIZE - 1);
        self.0[addr..addr + 4].copy_from_slice(&value.to_be_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KSEG0_BASE: usize = 0x80000000;

    /// Test to ensure every FastMemory::init properly maps every virtual
    /// memory page with the correct optional raw pointer value. 
    #[test]
    fn test_init_maps_pages_correctly() {
        let mem = FastMemory::new();

        assert_eq!(mem.vmem_lookup.len(), 0x100000);

        // No pages mapped in KUSEG area
        for kuseg_page in 0x00000..=0x7FFFF {
            assert!(mem.vmem_lookup[kuseg_page].is_none());
        }

        // No pages mapped in KSSEG area
        for ksseg_page in 0xC0000..=0xDFFFF {
            assert!(mem.vmem_lookup[ksseg_page].is_none());
        }

        // No pages mapped in KSEG3 area
        for kseg3_page in 0xE0000..=0xFFFFF {
            assert!(mem.vmem_lookup[kseg3_page].is_none());
        }

        for kseg0_page in 0x80000..=0x9FFFF {
            // All pages of RDRAM properly mapped in KSEG0 area
            if (0x80000..=0x807FF).contains(&kseg0_page) {
                assert!(mem.vmem_lookup[kseg0_page].is_some());

                let rdram_ptr = (&mem.rdram.0[(kseg0_page - 0x80000) * FastMemory::VPAGE_SIZE]) as *const u8;

                assert_eq!(
                    mem.vmem_lookup[kseg0_page].unwrap().as_ptr() as *const u8,
                    rdram_ptr,
                );
            }
            // All pages of DMEM properly mapped in KSEG0 area
            else if kseg0_page == 0x84000 {
                assert!(mem.vmem_lookup[kseg0_page].is_some());

                let dmem_ptr = (&mem.sp_dmem[(kseg0_page - 0x84000) * FastMemory::VPAGE_SIZE]) as *const u8;

                assert_eq!(
                    mem.vmem_lookup[kseg0_page].unwrap().as_ptr() as *const u8,
                    dmem_ptr,
                );
            }
            // All pages of IMEM properly mapped in KSEG0 area
            else if kseg0_page == 0x84001 {
                assert!(mem.vmem_lookup[kseg0_page].is_some());

                let imem_ptr = (&mem.sp_imem[(kseg0_page - 0x84001) * FastMemory::VPAGE_SIZE]) as *const u8;

                assert_eq!(
                    mem.vmem_lookup[kseg0_page].unwrap().as_ptr() as *const u8,
                    imem_ptr,
                );
            } else {
                assert!(mem.vmem_lookup[kseg0_page].is_none());
            }
        }

        for kseg1_page in 0xA0000..=0xBFFFF {
            // All pages of RDRAM properly mapped in KSEG1 area
            if (0xA0000..=0xA07FF).contains(&kseg1_page) {
                assert!(mem.vmem_lookup[kseg1_page].is_some());

                let rdram_ptr = (&mem.rdram.0[(kseg1_page - 0xA0000) * FastMemory::VPAGE_SIZE]) as *const u8;

                assert_eq!(
                    mem.vmem_lookup[kseg1_page].unwrap().as_ptr() as *const u8,
                    rdram_ptr,
                );
            }
            // All pages of DMEM properly mapped in KSEG1 area
            else if kseg1_page == 0xA4000 {
                assert!(mem.vmem_lookup[kseg1_page].is_some());

                let dmem_ptr = (&mem.sp_dmem[(kseg1_page - 0xA4000) * FastMemory::VPAGE_SIZE]) as *const u8;

                assert_eq!(
                    mem.vmem_lookup[kseg1_page].unwrap().as_ptr() as *const u8,
                    dmem_ptr,
                );
            }
            // All pages of IMEM properly mapped in KSEG1 area
            else if kseg1_page == 0xA4001 {
                assert!(mem.vmem_lookup[kseg1_page].is_some());

                let imem_ptr = (&mem.sp_imem[(kseg1_page - 0xA4001) * FastMemory::VPAGE_SIZE]) as *const u8;

                assert_eq!(
                    mem.vmem_lookup[kseg1_page].unwrap().as_ptr() as *const u8,
                    imem_ptr,
                );
            } else {
                assert!(mem.vmem_lookup[kseg1_page].is_none());
            }
        }
    }

    /// Test to ensure reads and writes through FastMemory::get_raw_mem work
    /// and alter the expected memory by verifying directly against the arrays.
    #[test]
    fn test_fastmem_read_write_access() {
        let mut mem = FastMemory::new();

        for i in 0..0x8000 {
            let vaddr = KSEG0_BASE + i;
            let rdram_val = mem.get_raw_mem(vaddr as u32).unwrap();
            unsafe {
                rdram_val.write(i as u8);
            }
        }

        for i in 0..0x8000 {
            let vaddr = KSEG0_BASE + i;
            let val = mem.get_raw_mem(vaddr as u32).unwrap();
            let val = unsafe { val.read() };
            assert_eq!(val, i as u8);
            assert_eq!(val, mem.rdram.0[i]);
        }

        println!("First 0x20 of RDRAM: {:?}", &mem.rdram.0[..0x20]);

        for i in 0..0x1000 {
            let vaddr = KSEG0_BASE + 0x04000000 + i;
            let dmem_val = mem.get_raw_mem(vaddr as u32).unwrap();
            unsafe {
                dmem_val.write((i as u8).wrapping_add(1));
            }
        }

        for i in 0..0x1000 {
            let vaddr = KSEG0_BASE + 0x04000000 + i;
            let dmem_val = mem.get_raw_mem(vaddr as u32).unwrap();
            let val = unsafe { dmem_val.read() };
            assert_eq!(val, (i as u8).wrapping_add(1));
            assert_eq!(val, mem.sp_dmem[i]);
        }

        println!("First 0x20 of D-Mem: {:?}", &mem.sp_dmem[..0x20]);

        for i in 0..0x1000 {
            let vaddr = KSEG0_BASE + 0x04001000 + i;
            let imem_val = mem.get_raw_mem(vaddr as u32).unwrap();
            unsafe {
                imem_val.write((i as u8).wrapping_add(2));
            }
        }

        for i in 0..0x1000 {
            let vaddr = KSEG0_BASE + 0x04001000 + i;
            let imem_val = mem.get_raw_mem(vaddr as u32).unwrap();
            let val = unsafe { imem_val.read() };
            assert_eq!(val, (i as u8).wrapping_add(2));
            assert_eq!(val, mem.sp_imem[i]);
        }

        println!("First 0x20 of I-Mem: {:?}", &mem.sp_imem[..0x20]);
    }
}
