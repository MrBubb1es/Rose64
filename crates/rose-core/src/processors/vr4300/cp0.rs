use crate::processors::vr4300::CpuException;

#[derive(Default)]
pub struct Cp0 {
    regs: [u64; 32],
}

impl Cp0 {
    pub fn index(&self)        -> u32 { self.regs[0] as u32 }
    pub fn random(&self)       -> u32 { self.regs[1] as u32 }
    pub fn entry_lo0(&self)    -> u64 { self.regs[2] }
    pub fn entry_lo1(&self)    -> u64 { self.regs[3] }
    pub fn context(&self)      -> u64 { self.regs[4] }
    pub fn page_mask(&self)    -> u32 { self.regs[5] as u32 }
    pub fn wired(&self)        -> u32 { self.regs[6] as u32 }

    pub fn bad_vaddr(&self)    -> u64 { self.regs[8] }
    pub fn count(&self)        -> u32 { self.regs[9] as u32 }
    pub fn entry_hi(&self)     -> u64 { self.regs[10] }
    pub fn compare(&self)      -> u32 { self.regs[11] as u32 }
    pub fn status(&self)       -> u32 { self.regs[12] as u32 }
    pub fn cause(&self)        -> u32 { self.regs[13] as u32 }
    pub fn epc(&self)          -> u64 { self.regs[14] }
    pub fn prid(&self)         -> u32 { self.regs[15] as u32 }
    pub fn config(&self)       -> u32 { self.regs[16] as u32 }
    pub fn lladdr(&self)       -> u32 { self.regs[17] as u32 }
    pub fn watch_lo(&self)     -> u32 { self.regs[18] as u32 }
    pub fn watch_hi(&self)     -> u32 { self.regs[19] as u32 }
    pub fn xcontext(&self)     -> u64 { self.regs[20] }

    pub fn parity_error(&self) -> u32 { self.regs[26] as u32 }
    pub fn cache_error(&self)  -> u32 { self.regs[27] as u32 }
    pub fn tag_lo(&self)       -> u32 { self.regs[28] as u32 }
    pub fn tag_hi(&self)       -> u32 { self.regs[29] as u32 }
    pub fn error_epc(&self)    -> u64 { self.regs[24] }

    pub fn set_index(&mut self, value: u32)        { self.regs[0]  = value as u64; }
    pub fn set_random(&mut self, value: u32)       { self.regs[1]  = value as u64; }
    pub fn set_entry_lo0(&mut self, value: u64)    { self.regs[2]  = value; }
    pub fn set_entry_lo1(&mut self, value: u64)    { self.regs[3]  = value; }
    pub fn set_context(&mut self, value: u64)      { self.regs[4]  = value; }
    pub fn set_page_mask(&mut self, value: u32)    { self.regs[5]  = value as u64; }
    pub fn set_wired(&mut self, value: u32)        { self.regs[6]  = value as u64; }
    pub fn set_bad_vaddr(&mut self, value: u64)    { self.regs[8]  = value; }
    pub fn set_count(&mut self, value: u32)        { self.regs[9]  = value as u64; }
    pub fn set_entry_hi(&mut self, value: u64)     { self.regs[10] = value; }
    pub fn set_compare(&mut self, value: u32)      { self.regs[11] = value as u64; }
    pub fn set_status(&mut self, value: u32)       { self.regs[12] = value as u64; }
    pub fn set_cause(&mut self, value: u32)        { self.regs[13] = value as u64; }
    pub fn set_epc(&mut self, value: u64)          { self.regs[14] = value; }
    pub fn set_prid(&mut self, value: u32)         { self.regs[15] = value as u64; }
    pub fn set_config(&mut self, value: u32)       { self.regs[16] = value as u64; }
    pub fn set_lladdr(&mut self, value: u32)       { self.regs[17] = value as u64; }
    pub fn set_watch_lo(&mut self, value: u32)     { self.regs[18] = value as u64; }
    pub fn set_watch_hi(&mut self, value: u32)     { self.regs[19] = value as u64; }
    pub fn set_xcontext(&mut self, value: u64)     { self.regs[20] = value; }
    pub fn set_parity_error(&mut self, value: u32) { self.regs[26] = value as u64; }
    pub fn set_cache_error(&mut self, value: u32)  { self.regs[27] = value as u64; }
    pub fn set_tag_lo(&mut self, value: u32)       { self.regs[28] = value as u64; }
    pub fn set_tag_hi(&mut self, value: u32)       { self.regs[29] = value as u64; }
    pub fn set_error_epc(&mut self, value: u64)    { self.regs[24] = value; }

    pub fn read(&self, idx: usize) -> u64 {
        match idx {
            1 => rand::random_range(self.wired() as u64 & 0x1F..=0x1F),
            _ => self.regs[idx],
        }
    }

    pub fn write32(&mut self, idx: usize, val: u32) -> Result<(), CpuException> {
        match idx {
            _ => self.regs[idx] = val as u64,
        }
        Ok(())
    }
}