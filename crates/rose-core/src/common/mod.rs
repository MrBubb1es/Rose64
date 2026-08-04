pub mod consts {
    pub const KB: usize = 1024;
    pub const MB: usize = 1024 * 1024;
}

pub mod hint {
    #[inline(always)]
    pub fn rose_likely(cond: bool) -> bool {
        if !cond {
            std::hint::cold_path();
        }
        cond
    }

    #[inline(always)]
    pub fn rose_unlikely(cond: bool) -> bool {
        if cond {
            std::hint::cold_path();
        }
        cond
    }
}