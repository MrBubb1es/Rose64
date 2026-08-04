pub mod consts {
    pub const KB: usize = 1024;
    pub const MB: usize = 1024 * KB;
    pub const GB: usize = 1024 * MB;
}

pub mod hint {
    /// Hint to the compiler that the given condition is likely to be true.
    #[inline(always)]
    pub fn rose_likely(cond: bool) -> bool {
        if !cond {
            std::hint::cold_path();
        }
        cond
    }

    /// Hint to the compiler that the given condition is likely to be false.
    #[inline(always)]
    pub fn rose_unlikely(cond: bool) -> bool {
        if cond {
            std::hint::cold_path();
        }
        cond
    }
}
