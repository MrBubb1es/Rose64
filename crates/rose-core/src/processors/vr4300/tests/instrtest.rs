//! -----------------------------------------------------------------------
//! instrtest.rs: Individual tests for VR4300 instructions.
//!
//! Contains tests for all VR4300 instructions. We try to test happy paths and
//! unhappy (exception-causing) paths, and any edge cases that may arise.
//!
//! Author(s): MrBubblezsz, logocrazymon
//! -----------------------------------------------------------------------

#![cfg(test)]

mod util {
    use crate::{
        memory::bus::Bus,
        processors::vr4300::{CpuException, CpuVR4300, RegSize},
    };

    /// KSEG0 base address of RDRAM (matches `test_unaligned_address_exceptions`).
    pub const RDRAM_BASE: u64 = 0xFFFFFFFF_80000000;

    /// Byte written into a store's target window before execution, so we can
    /// detect both "did the write happen" and, for SWL/SWR/SDL/SDR, exercise
    /// the byte-merge against a known "existing memory" pattern.
    pub const STORE_SENTINEL: u8 = 0xEE;

    macro_rules! itype_fail_str {
        () => {
            r#"
    {}:
        Instruction:
            Full = 0x{:08X}
            Op = 0b{:06b}
            rs = 0b{:05b}
            rt = 0b{:05b}
            imm = 0x{:04X}
        Input:
            Mode = {:?}
            rs = {}, GPR[rs] = {:016X}
            rt = {}, GPR[rt] = {:016X}
        Expected:
            GPR[rt] = {:016X}
            Exception = {:?}
        Got:
            GPR[rt] = {:016X}
            Exception = {:?}"#
        };
    }

    macro_rules! rtype_fail_str {
        () => {
            r#"
    {}:
        Instruction:
            Full = 0x{:08X}
            Op = 0b{:06b}
            rs = 0b{:05b}
            rt = 0b{:05b}
            rd = 0b{:05b}
            sa = 0b{:05b}
            func = 0b{:06b}
        Input:
            Mode = {:?}
            rs = {}, GPR[rs] = {:016X}
            rt = {}, GPR[rt] = {:016X}
            rd = {}, GPR[rd] = {:016X}
        Expected:
            GPR[rd] = {:016X}
            Exception = {:?}
        Got:
            GPR[rd] = {:016X}
            Exception = {:?}"#
        };
    }

    macro_rules! divmul_fail_str {
        () => {
            r#"
    {}:
        Instruction:
            Full = 0x{:08X}
            Op = 0b{:06b}
            rs = 0b{:05b}
            rt = 0b{:05b}
            rd = 0b00000
            sa = 0b00000
            func = 0b{:06b}
        Input:
            Mode = {:?}
            rs = {}, GPR[rs] = {:016X}
            rt = {}, GPR[rt] = {:016X}
            rd = 0, GPR[rd] = 0000000000000000
            HI = {:016X}
            LO = {:016X}
        Expected:
            HI = {:016X}
            LO = {:016X}
            Exception = {:?}
        Got:
            HI = {:016X}
            LO = {:016X}
            Exception = {:?}"#
        };
    }

    macro_rules! load_fail_str {
        () => {
            r#"
{}:
    Instruction:
        Full = 0x{:08X}
        Op = 0b{:06b}
        rs = 0b{:05b}
        rt = 0b{:05b}
        imm = 0x{:04X}
    Input:
        Mode = {:?}
        rs = {}, GPR[rs] (base) = 0x{:016X}
        Effective Address = 0x{:016X}
        rt = {}, GPR[rt] (before) = 0x{:016X}
    Expected:
        GPR[rt] = 0x{:016X}
        Exception = {:?}
    Got:
        GPR[rt] = 0x{:016X}
        Exception = {:?}"#
        };
    }

    macro_rules! store_fail_str {
        () => {
            r#"
{}:
    Instruction:
        Full = 0x{:08X}
        Op = 0b{:06b}
        rs = 0b{:05b}
        rt = 0b{:05b}
        imm = 0x{:04X}
    Input:
        Mode = {:?}
        rs = {}, GPR[rs] (base) = 0x{:016X}
        Effective Address = 0x{:016X}
        rt = {}, GPR[rt] (store value) = 0x{:016X}
    Expected:
        Memory[addr..] = {:02X?}
        Exception = {:?}
    Got:
        Memory[addr..] = {:02X?}
        Exception = {:?}"#
        };
    }

    /// Builds an I-type isntruction from its components. See ITypeInstruciton
    /// struct for the layout diagram.
    pub fn itype_instr(op: u32, rs: u32, rt: u32, immediate: u32) -> u32 {
        (op << 26) | (rs << 21) | (rt << 16) | immediate
    }

    /// Builds an R-type isntruction from its components. See RTypeInstruciton
    /// struct for the layout diagram.
    pub fn rtype_instr(op: u32, rs: u32, rt: u32, rd: u32, sa: u32, func: u32) -> u32 {
        (op << 26) | (rs << 21) | (rt << 16) | (rd << 11) | (sa << 6) | func
    }

    pub fn test_rom() -> Vec<u8> {
        use crate::common::consts::MB;

        vec![0; 8 * MB]
    }

    /// Test the execution of an I-Type instruction. Checks register and
    /// exception output vs. expected.
    #[allow(clippy::too_many_arguments)]
    pub fn test_itype_instr(
        name: &str,
        op: u32,
        rs: u32,
        rt: u32,
        rs_in: u64,
        rt_in: u64,
        immediate: u16,
        expected_rt_out: u64,
        expected_exception: Option<CpuException>,
        reg_size: Option<RegSize>,
    ) {
        // If reg_size not given, test both.
        if reg_size.is_none() {
            test_itype_instr(
                name,
                op,
                rs,
                rt,
                rs_in,
                rt_in,
                immediate,
                expected_rt_out,
                expected_exception,
                Some(RegSize::Reg32),
            );
            test_itype_instr(
                name,
                op,
                rs,
                rt,
                rs_in,
                rt_in,
                immediate,
                expected_rt_out,
                expected_exception,
                Some(RegSize::Reg64),
            );
            return;
        }

        let reg_size = reg_size.unwrap();

        let mut cpu = CpuVR4300::new();
        let mut bus = Bus::new(test_rom()).unwrap();
        let instr = itype_instr(op, rs, rt, immediate as u32);

        cpu.reg_size = reg_size;
        cpu.gpr[rs as usize] = rs_in;
        cpu.gpr[rt as usize] = rt_in;
        let exception = cpu.execute_instruction(&mut bus, instr).err();

        let rt_out = cpu.gpr[rt as usize];

        assert_eq!(
            exception,
            expected_exception,
            itype_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            immediate,
            reg_size,
            rs,
            rs_in,
            rt,
            rt_in,
            expected_rt_out,
            expected_exception,
            rt_out,
            exception
        );

        assert_eq!(
            rt_out,
            expected_rt_out,
            itype_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            immediate,
            reg_size,
            rs,
            rs_in,
            rt,
            rt_in,
            expected_rt_out,
            expected_exception,
            rt_out,
            exception
        );
    }

    /// Test the execution of an R-Type instruction. Checks register and
    /// exception output vs. expected.
    #[allow(clippy::too_many_arguments)]
    pub fn test_rtype_instr(
        name: &str,
        op: u32,
        rs: u32,
        rt: u32,
        rd: u32,
        sa: u32,
        func: u32,
        rs_in: u64,
        rt_in: u64,
        rd_in: u64,
        expected_rd_out: u64,
        expected_exception: Option<CpuException>,
        reg_size: Option<RegSize>,
    ) {
        // If reg_size not given, test both.
        if reg_size.is_none() {
            test_rtype_instr(
                name,
                op,
                rs,
                rt,
                rd,
                sa,
                func,
                rs_in,
                rt_in,
                rd_in,
                expected_rd_out,
                expected_exception,
                Some(RegSize::Reg32),
            );
            test_rtype_instr(
                name,
                op,
                rs,
                rt,
                rd,
                sa,
                func,
                rs_in,
                rt_in,
                rd_in,
                expected_rd_out,
                expected_exception,
                Some(RegSize::Reg64),
            );
            return;
        }

        let reg_size = reg_size.unwrap();

        let mut cpu = CpuVR4300::new();
        let mut bus = Bus::new(test_rom()).unwrap();
        let instr = rtype_instr(op, rs, rt, rd, sa, func);

        cpu.reg_size = reg_size;
        cpu.gpr[rs as usize] = rs_in;
        cpu.gpr[rt as usize] = rt_in;
        cpu.gpr[rd as usize] = rd_in;
        let exception = cpu.execute_instruction(&mut bus, instr).err();

        let rd_out = cpu.gpr[rd as usize];

        assert_eq!(
            exception,
            expected_exception,
            rtype_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            rd,
            sa,
            func,
            reg_size,
            rs,
            rs_in,
            rt,
            rt_in,
            rd,
            rd_in,
            expected_rd_out,
            expected_exception,
            rd_out,
            exception
        );

        assert_eq!(
            rd_out,
            expected_rd_out,
            rtype_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            rd,
            sa,
            func,
            reg_size,
            rs,
            rs_in,
            rt,
            rt_in,
            rd,
            rd_in,
            expected_rd_out,
            expected_exception,
            rd_out,
            exception
        );
    }

    /// Test the execution of a division or multiplication instruction.
    #[allow(clippy::too_many_arguments)]
    pub fn test_divmul_instr(
        name: &str,
        op: u32,
        rs: u32,
        rt: u32,
        func: u32,
        rs_in: u64,
        rt_in: u64,
        hi_in: u64,
        lo_in: u64,
        expected_hi_out: u64,
        expected_lo_out: u64,
        expected_exception: Option<CpuException>,
        reg_size: RegSize,
    ) {
        let mut cpu = CpuVR4300::new();
        let mut bus = Bus::new(test_rom()).unwrap();
        let instr = rtype_instr(op, rs, rt, 0, 0, func);

        cpu.reg_size = reg_size;
        cpu.gpr[rs as usize] = rs_in;
        cpu.gpr[rt as usize] = rt_in;
        cpu.mult_hi = hi_in;
        cpu.mult_lo = hi_in;
        let exception = cpu.execute_instruction(&mut bus, instr).err();

        assert_eq!(
            exception,
            expected_exception,
            divmul_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            func,
            reg_size,
            rs,
            rs_in,
            rt,
            rt_in,
            hi_in,
            lo_in,
            expected_hi_out,
            expected_lo_out,
            expected_exception,
            cpu.mult_hi,
            cpu.mult_lo,
            exception
        );

        assert_eq!(
            cpu.mult_hi,
            expected_hi_out,
            divmul_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            func,
            reg_size,
            rs,
            rs_in,
            rt,
            rt_in,
            hi_in,
            lo_in,
            expected_hi_out,
            expected_lo_out,
            expected_exception,
            cpu.mult_hi,
            cpu.mult_lo,
            exception
        );

        assert_eq!(
            cpu.mult_lo,
            expected_lo_out,
            divmul_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            func,
            reg_size,
            rs,
            rs_in,
            rt,
            rt_in,
            hi_in,
            lo_in,
            expected_hi_out,
            expected_lo_out,
            expected_exception,
            cpu.mult_hi,
            cpu.mult_lo,
            exception
        );
    }

    /// Computes `base + sign_extend(imm)`, matching how the CPU forms the
    /// effective address for all load/store instructions.
    pub fn eff_addr(base: u64, immediate: u16) -> u64 {
        base.wrapping_add(((immediate as i16) as i64) as u64)
    }

    /// Test the execution of a load instruction. `mem_addr` is the absolute
    /// address at which `mem_bytes` is written into RDRAM before execution
    /// (equal to the effective address for simple loads; the aligned
    /// word/dword address for LWL/LWR/LDL/LDR).
    #[allow(clippy::too_many_arguments)]
    pub fn test_load_instr(
        name: &str,
        op: u32,
        rs: u32,
        rt: u32,
        base: u64,
        immediate: u16,
        rt_in: u64,
        mem_addr: u64,
        mem_bytes: &[u8],
        expected_rt_out: u64,
        expected_exception: Option<CpuException>,
        reg_size: Option<RegSize>,
    ) {
        if reg_size.is_none() {
            test_load_instr(
                name,
                op,
                rs,
                rt,
                base,
                immediate,
                rt_in,
                mem_addr,
                mem_bytes,
                expected_rt_out,
                expected_exception,
                Some(RegSize::Reg32),
            );
            test_load_instr(
                name,
                op,
                rs,
                rt,
                base,
                immediate,
                rt_in,
                mem_addr,
                mem_bytes,
                expected_rt_out,
                expected_exception,
                Some(RegSize::Reg64),
            );
            return;
        }

        let reg_size = reg_size.unwrap();

        let mut cpu = CpuVR4300::new();
        let mut bus = Bus::new(test_rom()).unwrap();
        let instr = itype_instr(op, rs, rt, immediate as u32);
        let addr = eff_addr(base, immediate);
        let window = (mem_addr - RDRAM_BASE) as usize;

        bus.memory.rdram.0[window..window + mem_bytes.len()].copy_from_slice(mem_bytes);

        cpu.reg_size = reg_size;
        cpu.gpr[rs as usize] = base;
        cpu.gpr[rt as usize] = rt_in;
        let exception = cpu.execute_instruction(&mut bus, instr).err();

        let rt_out = cpu.gpr[rt as usize];

        assert_eq!(
            exception,
            expected_exception,
            load_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            immediate,
            reg_size,
            rs,
            base,
            addr,
            rt,
            rt_in,
            expected_rt_out,
            expected_exception,
            rt_out,
            exception
        );

        assert_eq!(
            rt_out,
            expected_rt_out,
            load_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            immediate,
            reg_size,
            rs,
            base,
            addr,
            rt,
            rt_in,
            expected_rt_out,
            expected_exception,
            rt_out,
            exception
        );
    }

    /// Like `test_load_instr`, but also checks `cpu.llbit` (for LL/LLD).
    /// `llbit` is reset to `false` before execution.
    #[allow(clippy::too_many_arguments)]
    pub fn test_ll_instr(
        name: &str,
        op: u32,
        rs: u32,
        rt: u32,
        base: u64,
        immediate: u16,
        rt_in: u64,
        mem_bytes: &[u8],
        expected_rt_out: u64,
        expected_exception: Option<CpuException>,
        expected_llbit: bool,
        reg_size: Option<RegSize>,
    ) {
        if reg_size.is_none() {
            test_ll_instr(
                name,
                op,
                rs,
                rt,
                base,
                immediate,
                rt_in,
                mem_bytes,
                expected_rt_out,
                expected_exception,
                expected_llbit,
                Some(RegSize::Reg32),
            );
            test_ll_instr(
                name,
                op,
                rs,
                rt,
                base,
                immediate,
                rt_in,
                mem_bytes,
                expected_rt_out,
                expected_exception,
                expected_llbit,
                Some(RegSize::Reg64),
            );
            return;
        }

        let reg_size = reg_size.unwrap();

        let mut cpu = CpuVR4300::new();
        let mut bus = Bus::new(test_rom()).unwrap();
        let instr = itype_instr(op, rs, rt, immediate as u32);
        let addr = eff_addr(base, immediate);
        let window = (addr - RDRAM_BASE) as usize;

        bus.memory.rdram.0[window..window + mem_bytes.len()].copy_from_slice(mem_bytes);

        cpu.reg_size = reg_size;
        cpu.gpr[rs as usize] = base;
        cpu.gpr[rt as usize] = rt_in;
        cpu.llbit = false;
        let exception = cpu.execute_instruction(&mut bus, instr).err();

        let rt_out = cpu.gpr[rt as usize];

        assert_eq!(
            exception,
            expected_exception,
            load_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            immediate,
            reg_size,
            rs,
            base,
            addr,
            rt,
            rt_in,
            expected_rt_out,
            expected_exception,
            rt_out,
            exception
        );

        assert_eq!(
            rt_out,
            expected_rt_out,
            load_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            immediate,
            reg_size,
            rs,
            base,
            addr,
            rt,
            rt_in,
            expected_rt_out,
            expected_exception,
            rt_out,
            exception
        );

        assert_eq!(
            cpu.llbit, expected_llbit,
            "{}: expected llbit = {}, got llbit = {}",
            name, expected_llbit, cpu.llbit
        );
    }

    /// Test the execution of a store instruction. Pre-fills the `expected_mem`-
    /// sized window at `mem_addr` with `STORE_SENTINEL`, executes, and compares
    /// the resulting bytes and exception against expectations.
    #[allow(clippy::too_many_arguments)]
    pub fn test_store_instr(
        name: &str,
        op: u32,
        rs: u32,
        rt: u32,
        base: u64,
        immediate: u16,
        rt_in: u64,
        mem_addr: u64,
        expected_mem: &[u8],
        expected_exception: Option<CpuException>,
        reg_size: Option<RegSize>,
    ) {
        if reg_size.is_none() {
            test_store_instr(
                name,
                op,
                rs,
                rt,
                base,
                immediate,
                rt_in,
                mem_addr,
                expected_mem,
                expected_exception,
                Some(RegSize::Reg32),
            );
            test_store_instr(
                name,
                op,
                rs,
                rt,
                base,
                immediate,
                rt_in,
                mem_addr,
                expected_mem,
                expected_exception,
                Some(RegSize::Reg64),
            );
            return;
        }

        let reg_size = reg_size.unwrap();

        let mut cpu = CpuVR4300::new();
        let mut bus = Bus::new(test_rom()).unwrap();
        let instr = itype_instr(op, rs, rt, immediate as u32);
        let addr = eff_addr(base, immediate);
        let window = (mem_addr - RDRAM_BASE) as usize;

        for b in bus.memory.rdram.0[window..window + expected_mem.len()].iter_mut() {
            *b = STORE_SENTINEL;
        }

        cpu.reg_size = reg_size;
        cpu.gpr[rs as usize] = base;
        cpu.gpr[rt as usize] = rt_in;
        let exception = cpu.execute_instruction(&mut bus, instr).err();

        let actual_mem = bus.memory.rdram.0[window..window + expected_mem.len()].to_vec();

        assert_eq!(
            exception,
            expected_exception,
            store_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            immediate,
            reg_size,
            rs,
            base,
            addr,
            rt,
            rt_in,
            expected_mem,
            expected_exception,
            actual_mem,
            exception
        );

        assert_eq!(
            actual_mem,
            expected_mem,
            store_fail_str!(),
            name,
            instr,
            op,
            rs,
            rt,
            immediate,
            reg_size,
            rs,
            base,
            addr,
            rt,
            rt_in,
            expected_mem,
            expected_exception,
            actual_mem,
            exception
        );
    }
}

mod alu_instructions {
    use super::util::*;
    use crate::processors::vr4300::{CpuException, RegSize};

    /// Test the ADD instruction.
    ///
    /// # ADD:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rd] <- GPR[rs] + GPR[rt]`
    /// - 64-bit:
    ///   - `temp    <- GPR[rs] + GPR[rt]`
    ///   - `GPR[rd] <- sign_extend_u64::<32>(temp)`
    /// ## Exceptions:
    /// - Integer Overflow
    #[test]
    fn test_add() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100000;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let rs_in: u64 = 0x00000000_1001FEDC;
        let rt_in: u64 = 0xFFFFFFFF_81234567;
        let rd_out: u64 = ((rs_in as i32) + (rt_in as i32)) as u64;

        test_rtype_instr(
            "ADD", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );

        // Overflow test (output register unchanged)
        let rs_in: u64 = 0xFFFFFFFF_FFFFFFFF;
        let rt_in: u64 = 0xFFFFFFFF_80000000;
        let rd_out: u64 = rd_in;

        test_rtype_instr(
            "ADD",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            Some(CpuException::ArithmeticOverflow),
            None,
        );
    }

    /// Test the ADDI instruction.
    ///
    /// # ADDI:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rt] <- GRP[rs] + sign_extend_u32::<16>(immediate)`
    /// - 64-bit:
    ///   - `temp    <- GPR[rs] + sign_extend_u64::<16>(immediate)`
    ///   - `GPR[rt] <- sign_extend_u64::<32>(temp)`
    /// ## Exceptions:
    /// - Integer Overflow
    #[test]
    fn test_addi() {
        const OP: u32 = 0b001000;

        let rs: u32 = 4;
        let rt: u32 = 5;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let immediate: u16 = 0x0ACE;
        let rs_in: u64 = 0x00000000_0ACE0987;
        let rt_out: u64 = (((immediate as i16) as i32) + (rs_in as i32)) as u64;

        test_itype_instr(
            "ADDI", OP, rs, rt, rs_in, rt_in, immediate, rt_out, None, None,
        );

        // Overflow test (output register unchanged)
        let immediate: u16 = 0x7FFF;
        let rs_in: u64 = 0x00000000_7FFFFFFF;
        let rt_out: u64 = rt_in;

        test_itype_instr(
            "ADDI",
            OP,
            rs,
            rt,
            rs_in,
            rt_in,
            immediate,
            rt_out,
            Some(CpuException::ArithmeticOverflow),
            None,
        );
    }

    /// Test the ADDIU instruction.
    ///
    /// # ADDIU:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rt] <- GRP[rs] + sign_extend_u32::<16>(immediate)`
    /// - 64-bit:
    ///   - `temp    <- GPR[rs] + sign_extend_u64::<16>(immediate)`
    ///   - `GPR[rt] <- sign_extend_u64::<32>(temp)`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_addiu() {
        const OP: u32 = 0b001001;

        let rs: u32 = 4;
        let rt: u32 = 5;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let immediate: u16 = 0x0ACE;
        let rs_in: u64 = 0x00000000_0ACE0987;
        let rt_out: u64 = (((immediate as i16) as i32) + (rs_in as i32)) as u64;

        test_itype_instr(
            "ADDIU", OP, rs, rt, rs_in, rt_in, immediate, rt_out, None, None,
        );

        // Overflow test
        let immediate: u16 = 0x7FFF;
        let rs_in: u64 = 0x00000000_7FFFFFFF;
        let rt_out: u64 = (((immediate as i16) as i32).wrapping_add(rs_in as i32)) as u64;

        test_itype_instr(
            "ADDI", OP, rs, rt, rs_in, rt_in, immediate, rt_out, None, None,
        );
    }

    /// Test the ADDU instruction.
    ///
    /// # ADDU:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rd] <- GPR[rs] + GPR[rt]`
    /// - 64-bit:
    ///   - `temp    <- GPR[rs] + GPR[rt]`
    ///   - `GPR[rd] <- sign_extend_u64::<32>(temp)`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_addu() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100001;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let rs_in: u64 = 0x00000000_1001FEDC;
        let rt_in: u64 = 0xFFFFFFFF_81234567;
        let rd_out: u64 = ((rs_in as i32).wrapping_add(rt_in as i32)) as u64;

        test_rtype_instr(
            "ADDU", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );

        // Overflow test
        let rs_in: u64 = 0xFFFFFFFF_FFFFFFFF;
        let rt_in: u64 = 0xFFFFFFFF_80000000;
        let rd_out: u64 = ((rs_in as i32).wrapping_add(rt_in as i32)) as u64;

        test_rtype_instr(
            "ADDU", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the AND instruction.
    ///
    /// # AND:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - `GPR[rd] <- GPR[rs] + GPR[rt]`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_and() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100100;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;

        let rs_in: u64 = 0xDEADC0DE_0123FEDC;
        let rt_in: u64 = 0xCAFEF00D_01230000;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let rd_out: u64 = rs_in & rt_in;

        test_rtype_instr(
            "AND", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the ANDI instruction.
    ///
    /// # ANDI:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - `GPR[rt] <- imm || GPR[rs]`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_andi() {
        const OP: u32 = 0b001100;

        let rs: u32 = 4;
        let rt: u32 = 5;

        let rs_in: u64 = 0xABCDEF01_23456789;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let immediate: u16 = 0xFEED;
        let rt_out: u64 = rs_in & immediate as u64;

        test_itype_instr(
            "ANDI", OP, rs, rt, rs_in, rt_in, immediate, rt_out, None, None,
        );
    }

    /// Test the NOR instruction.
    ///
    /// # NOR:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - `GPR[rd] <- GPR[rs] NOR GPR[rt]`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_nor() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100111;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;

        let rs_in: u64 = 0xDEADC0DE_0123FEDC;
        let rt_in: u64 = 0xCAFEF00D_01230000;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let rd_out: u64 = !(rs_in | rt_in);

        test_rtype_instr(
            "NOR", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the OR instruction.
    ///
    /// # OR:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - `GPR[rd] <- GPR[rs] | GPR[rt]`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_or() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100101;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;

        let rs_in: u64 = 0xDEADC0DE_0123FEDC;
        let rt_in: u64 = 0xCAFEF00D_01230000;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let rd_out: u64 = rs_in | rt_in;

        test_rtype_instr(
            "OR", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the ORI instruction.
    ///
    /// # ORI:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - `GPR[rt] <- imm | GPR[rs]`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_ori() {
        const OP: u32 = 0b001101;

        let rs: u32 = 4;
        let rt: u32 = 5;

        let rs_in: u64 = 0xABCDEF01_23456789;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let immediate: u16 = 0xFEED;
        let rt_out: u64 = rs_in | immediate as u64;

        test_itype_instr(
            "ORI", OP, rs, rt, rs_in, rt_in, immediate, rt_out, None, None,
        );
    }

    /// Test the XOR instruction.
    ///
    /// # XOR:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - `GPR[rd] <- GPR[rs] ^ GPR[rt]`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_xor() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100110;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;

        let rs_in: u64 = 0xDEADC0DE_0123FEDC;
        let rt_in: u64 = 0xCAFEF00D_01230000;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let rd_out: u64 = rs_in ^ rt_in;

        test_rtype_instr(
            "XOR", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the XORI instruction.
    ///
    /// # ANDI:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - GPR[rt] <- imm ^ GPR[rs]
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_xori() {
        const OP: u32 = 0b001110;

        let rs: u32 = 4;
        let rt: u32 = 5;

        let rs_in: u64 = 0xABCDEF01_23456789;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let immediate: u16 = 0xFEED;
        let rt_out: u64 = rs_in ^ immediate as u64;

        test_itype_instr(
            "XORI", OP, rs, rt, rs_in, rt_in, immediate, rt_out, None, None,
        );
    }

    /// Test the DADD instruction.
    ///
    /// # DADD:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - `GPR[rd] <- GPR[rs] + GPR[rt]`
    /// ## Exceptions:
    /// - Integer Overflow
    /// - Reserved Instruction
    #[test]
    fn test_dadd() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b101100;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DADD",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // No Overflow
        let rs_in: u64 = 0x1001FEDC_7F360123;
        let rt_in: u64 = 0x81234567_FEDCBA98;
        let rd_out: u64 = rs_in + rt_in;

        test_rtype_instr(
            "DADD",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );

        // Overflow test (output register unchanged)
        let rs_in: u64 = 0x7FFFFFFF_FFFFFFFF;
        let rt_in: u64 = 0x00000000_00000001;
        let rd_out: u64 = rd_in;

        test_rtype_instr(
            "DADD",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            Some(CpuException::ArithmeticOverflow),
            Some(RegSize::Reg64),
        );
    }

    /// Test the DADDI instruction.
    ///
    /// # DADDI:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - `GPR[rt] <- GPR[rs] + sign_extend_u64::<16>(imm)`
    /// ## Exceptions:
    /// - Integer Overflow
    /// - Reserved Instruction
    #[test]
    fn test_daddi() {
        const OP: u32 = 0b011000;

        let rs: u32 = 4;
        let rt: u32 = 5;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_itype_instr(
            "DADDI",
            OP,
            rs,
            rt,
            0u64,
            rt_in,
            0u16,
            rt_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // No overflow
        let immediate: u16 = 0x0ACE;

        let rs_in: u64 = 0x0ACE0987_DEFACE00;
        let rt_out: u64 = rs_in + (immediate as i16) as u64;

        test_itype_instr(
            "DADDI",
            OP,
            rs,
            rt,
            rs_in,
            rt_in,
            immediate,
            rt_out,
            None,
            Some(RegSize::Reg64),
        );

        // Overflow test (output register unchanged)
        let immediate: u16 = 0xFFFF;

        let rs_in: u64 = 0x80000000_00000000;
        let rt_out: u64 = rt_in;

        test_itype_instr(
            "DADDI",
            OP,
            rs,
            rt,
            rs_in,
            rt_in,
            immediate,
            rt_out,
            Some(CpuException::ArithmeticOverflow),
            Some(RegSize::Reg64),
        );
    }

    /// Test the DADDU instruction.
    ///
    /// # DADDU:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - `GPR[rd] <- GPR[rs] + GPR[rt]`
    /// ## Exceptions:
    /// - Reserved Instruction
    #[test]
    fn test_daddu() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b101101;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DADDU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // No Overflow
        let rs_in: u64 = 0x1001FEDC_7F360123;
        let rt_in: u64 = 0x81234567_FEDCBA98;
        let rd_out: u64 = rs_in + rt_in;

        test_rtype_instr(
            "DADDU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );

        // Overflow test (no exception should occur)
        let rs_in: u64 = 0x7FFFFFFF_FFFFFFFF;
        let rt_in: u64 = 0x00000000_00000001;
        let rd_out: u64 = rs_in.wrapping_add(rt_in);

        test_rtype_instr(
            "DADDU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );
    }

    /// Test the DADDIU instruction.
    ///
    /// # DADDIU:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - `GPR[rt] <- GPR[rs] + sign_extend_u64::<16>(imm)`
    /// ## Exceptions:
    /// - Reserved Instruction
    #[test]
    fn test_daddiu() {
        const OP: u32 = 0b011001;

        let rs: u32 = 4;
        let rt: u32 = 5;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_itype_instr(
            "DADDIU",
            OP,
            rs,
            rt,
            0u64,
            rt_in,
            0u16,
            rt_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // No overflow
        let immediate: u16 = 0x0ACE;

        let rs_in: u64 = 0x0ACE0987_DEFACE00;
        let rt_out: u64 = rs_in + (immediate as i16) as u64;

        test_itype_instr(
            "DADDIU",
            OP,
            rs,
            rt,
            rs_in,
            rt_in,
            immediate,
            rt_out,
            None,
            Some(RegSize::Reg64),
        );

        // Overflow test (output register unchanged)
        let immediate: u16 = 0xFFFF;

        let rs_in: u64 = 0x80000000_00000000;
        let rt_out: u64 = rs_in.wrapping_add((immediate as i16) as u64);

        test_itype_instr(
            "DADDIU",
            OP,
            rs,
            rt,
            rs_in,
            rt_in,
            immediate,
            rt_out,
            None,
            Some(RegSize::Reg64),
        );
    }

    /// Test the SUB instruction.
    ///
    /// # SUB:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rd] <- GPR[rs] + GPR[rt]`
    /// - 64-bit:
    ///   - temp    <- GPR[rs] + GPR[rt]
    ///   - `GPR[rd] <- sign_extend_u64::<32>(temp)`
    /// ## Exceptions:
    /// - Integer Overflow
    #[test]
    fn test_sub() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100010;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let rs_in: u64 = 5002;
        let rt_in: u64 = 963;
        let rd_out: u64 = rs_in - rt_in;

        test_rtype_instr(
            "SUB", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );

        // Overflow test (output register unchanged)
        let rs_in: u64 = i32::MIN as u64;
        let rt_in: u64 = 1;
        let rd_out: u64 = rd_in;

        test_rtype_instr(
            "SUB",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            Some(CpuException::ArithmeticOverflow),
            None,
        );
    }

    /// Test the SUBU instruction.
    ///
    /// # SUBU:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rd] <- GPR[rs] + GPR[rt]`
    /// - 64-bit:
    ///   - `temp    <- GPR[rs] + GPR[rt]`
    ///   - `GPR[rd] <- sign_extend_u64::<32>(temp)`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_subu() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100011;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let rs_in: u64 = 5002;
        let rt_in: u64 = 963;
        let rd_out: u64 = ((rs_in - rt_in) as i32) as u64;

        test_rtype_instr(
            "SUBU", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );

        // Overflow test (no exception occurs)
        let rs_in: u64 = i32::MIN as u64;
        let rt_in: u64 = 1;
        let rd_out: u64 = (rs_in.wrapping_sub(rt_in) as i32) as u64;

        test_rtype_instr(
            "SUBU", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the DSUB instruction.
    ///
    /// # DSUB:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - `GPR[rd] <- GPR[rs] - GPR[rt]`
    /// ## Exceptions:
    /// - Integer Overflow
    /// - Reserved Instruction
    #[test]
    fn test_dsub() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b101110;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DSUB",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // No overflow
        let rs_in: i64 = 5002;
        let rt_in: i64 = 963;
        let rd_out: i64 = rs_in - rt_in;

        test_rtype_instr(
            "DSUB",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in as u64,
            rt_in as u64,
            rd_in,
            rd_out as u64,
            None,
            Some(RegSize::Reg64),
        );

        // Overflow test (output register unchanged)
        let rs_in: i64 = i64::MIN;
        let rt_in: i64 = 1234;

        test_rtype_instr(
            "DSUB",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in as u64,
            rt_in as u64,
            rd_in,
            rd_in,
            Some(CpuException::ArithmeticOverflow),
            Some(RegSize::Reg64),
        );
    }

    /// Test the DSUBU instruction.
    ///
    /// # DSUBU:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - `GPR[rd] <- GPR[rs] - GPR[rt]`
    /// ## Exceptions:
    /// - Reserved Instruction
    #[test]
    fn test_dsubu() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b101111;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DSUBU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // No overflow
        let rs_in: i64 = 5002;
        let rt_in: i64 = 963;
        let rd_out: i64 = rs_in - rt_in;

        test_rtype_instr(
            "DSUBU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in as u64,
            rt_in as u64,
            rd_in,
            rd_out as u64,
            None,
            Some(RegSize::Reg64),
        );

        // Overflow test (no exception occurs)
        let rs_in: i64 = i64::MIN;
        let rt_in: i64 = 1234;
        let rd_out: i64 = rs_in.wrapping_sub(rt_in);

        test_rtype_instr(
            "DSUBU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in as u64,
            rt_in as u64,
            rd_in,
            rd_out as u64,
            None,
            Some(RegSize::Reg64),
        );
    }

    /// Test the SLL instruction.
    ///
    /// # SLL:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rd] <- GPR[rt] << sa`
    /// - 64-bit:
    ///   - `temp    <- GPR[rt] << sa`
    ///   - `GPR[rd] <- sign_extend_u64::<32>(temp)`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_sll() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b000000;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rs_in: u64 = 0x11111111_11111111; // unused for SLL
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // Regular shift
        let sa = 15;
        let rt_in: u64 = 0xFFFFFFFF_81234567;
        let rd_out: u64 = ((rt_in as i32) << sa) as u64;

        test_rtype_instr(
            "SLL", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );

        // Shift by 0 (should sign extend)
        let sa = 0;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out: u64 = (rt_in as i32) as u64;

        test_rtype_instr(
            "SLL", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the SLLV instruction.
    ///
    /// # SLLV:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rd] <- GPR[rt] << (GPR[rs] & 31)`
    /// - 64-bit:
    ///   - `temp    <- GPR[rt] << (GPR[rs] & 31)`
    ///   - `GPR[rd] <- sign_extend_u64::<32>(temp)`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_sllv() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b000100;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let sa: u32 = 0; // unused for SLLV
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // Regular shift
        let rs_in: u64 = 0xFFFFFFFF_FFFFFFFF;
        let rt_in: u64 = 0xFFFFFFFF_81234567;
        let rd_out: u64 = ((rt_in << (rs_in & 31)) as i32) as u64;

        test_rtype_instr(
            "SLLV", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );

        // Shift by 0 (should sign extend)
        let rs_in: u64 = 0x00000000_00000000;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out: u64 = (rt_in as i32) as u64;

        test_rtype_instr(
            "SLLV", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the DSLL instruction.
    ///
    /// # DSLL:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - `GPR[rd] <- GPR[rt] << sa`
    /// ## Exceptions:
    /// - Reserved Instruction
    #[test]
    fn test_dsll() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b111000;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rs_in: u64 = 0x11111111_11111111; // unused for DSLL
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DSLL",
            OP,
            rs,
            rt,
            rd,
            0u32,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // Regular shift
        let sa = 15;
        let rt_in: u64 = 0xFFFFFFFF_81234567;
        let rd_out: u64 = rt_in << sa;

        test_rtype_instr(
            "DSLL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );

        // 32 bit mode should raise an exception
        let sa = 0;
        let rt_in: u64 = 0x00000000_80000000;

        test_rtype_instr(
            "DSLL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );
    }

    /// Test the DSLLV instruction.
    ///
    /// # DSLLV:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - `GPR[rd] <- GPR[rt] << (GPR[rs] & 31)`
    /// ## Exceptions:
    /// - Reserved Instruction
    #[test]
    fn test_dsllv() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b010100;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let sa: u32 = 0; // unused for DSLLV
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DSLLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // Regular shift
        let rs_in: u64 = 0xFFFFFFFF_FFFFFFFF;
        let rt_in: u64 = 0xFFFFFFFF_81234567;
        let rd_out: u64 = (rt_in & 0xFFFFFFFF) << (rs_in & 0x3F);

        test_rtype_instr(
            "DSLLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );

        // 32 bit should throw an exception
        test_rtype_instr(
            "DSLLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );
    }

    /// Test the SRA instruction.
    ///
    /// # SRA:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rd] <- sign_extend::<32 - sa>(GPR[rt] >> sa)`
    /// - 64-bit:
    ///   - `temp    <- sign_extend::<32 - sa>(GPR[rt] >> sa)`
    ///   - `GPR[rd] <- sign_extend_u64::<32>(temp)`
    /// ## Exceptions:
    /// - None
    /// ## HARDWARE BUG (32-bit mode):
    ///   Instead of shifting in 1's or 0's based on the sign of the 32-bit
    ///   `GPR[rt]` value, bits from the high 32-bits are shifted in instead.
    #[test]
    fn test_sra() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b000011;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rs_in: u64 = 0x11111111_11111111; // unused for SRA
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // Regular shift
        let sa = 15;
        let rt_in: u64 = 0x00000000_81234567;
        // HARDWARE BUG (32-bit mode):
        //   Causes rd_out_32 to contain 0's in the high bits after the shift.
        //   As the 32-bit rt_in is signed, without the but there would be 1's.
        let rd_out: u64 = ((rt_in >> sa) as i32) as u64;

        test_rtype_instr(
            "SRA", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );

        // Shift by 0 (should sign extend)
        let sa = 0;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out: u64 = (rt_in as i32) as u64;

        test_rtype_instr(
            "SRA", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the SRAV instruction.
    ///
    /// # SRAV:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rd] <- sign_extend::<32 - sa>(GPR[rt] >> (GPR[rs] & 31))`
    /// - 64-bit:
    ///   - `temp    <- sign_extend::<32 - sa>(GPR[rt] >> (GPR[rs] & 31))`
    ///   - `GPR[rd] <- sign_extend_u64::<32>(temp)`
    /// ## Exceptions:
    /// - None
    /// ## HARDWARE BUG (32-bit mode):
    ///   Instead of shifting in 1's or 0's based on the sign of the 32-bit
    ///   `GPR[rt]` value, bits from the high 32-bits are shifted in instead.
    #[test]
    fn test_srav() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b000111;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let sa: u32 = 0; // unused for SRAV
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // Regular shift
        let rs_in: u64 = 0xFFFFFFFF_FFFFFFFF;
        let rt_in: u64 = 0x00000000_81234567;
        // HARDWARE BUG (32-bit mode):
        //   Causes rd_out_32 to contain 0's in the high bits after the shift.
        //   As the 32-bit rt_in is signed, without the but there would be 1's.
        let rd_out: u64 = (((rt_in as i64) >> (rs_in & 31)) as i32) as u64;

        test_rtype_instr(
            "SRAV", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );

        // Shift by 0 (should sign extend)
        let rs_in: u64 = 0x00000000_00000000;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out: u64 = (rt_in as i32) as u64;

        test_rtype_instr(
            "SRAV", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the DSRA instruction.
    ///
    /// # DSRA:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - `GPR[rd] <- sign_extend::<64 - sa>(GPR[rt] >> sa)`
    /// ## Exceptions:
    /// - Reserved Instruction
    #[test]
    fn test_dsra() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b111011;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rs_in: u64 = 0x11111111_11111111; // unused for DSRA
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DSRA",
            OP,
            rs,
            rt,
            rd,
            0u32,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // Regular shift
        let sa = 15;
        let rt_in: u64 = 0x00000000_81234567;
        let rd_out: u64 = ((rt_in as i64) >> sa) as u64;

        test_rtype_instr(
            "DSRA",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );

        // Shift by 0 (NOP in 64-bit mode)
        let sa = 0;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out: u64 = rt_in;

        test_rtype_instr(
            "DSRA",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );
    }

    /// Test the DSRAV instruction.
    ///
    /// # DSRAV:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - GPR[rd] <- sign_extend::<64 - sa>(GPR[rt] >> (GPR[rs] & 31))
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_dsrav() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b010111;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let sa: u32 = 0; // unused for DSRAV
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DSRAV",
            OP,
            rs,
            rt,
            rd,
            0u32,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // Regular shift
        let rs_in: u64 = 0xFFFFFFFF_FFFFFFFF;
        let rt_in: u64 = 0x00000000_81234567;
        let rd_out: u64 = ((rt_in as i64) >> (rs_in & 63)) as u64;

        test_rtype_instr(
            "DSRAV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );

        // Shift by 0 (NOP in 64-bit mode)
        let rs_in: u64 = 0x00000000_00000000;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out: u64 = rt_in;

        test_rtype_instr(
            "DSRAV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );
    }

    /// Test the DSRA32 instruction.
    ///
    /// # DSRA32:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - GPR[rd] <- sign_extend::<64 - (sa + 32)>(GPR[rt] >> (sa + 32))
    /// ## Exceptions:
    /// - Reserved Instruction
    #[test]
    fn test_dsra32() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b111111;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rs_in: u64 = 0x11111111_11111111; // unused for DSRA32
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DSRA32",
            OP,
            rs,
            rt,
            rd,
            0u32,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // Regular shift
        let sa = 15;
        let rt_in: u64 = 0x00000000_81234567;
        let rd_out: u64 = ((rt_in as i64) >> (sa + 32)) as u64;

        test_rtype_instr(
            "DSRA32",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );
    }

    /// Test the SRL instruction.
    ///
    /// # SRL:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rd] <- GPR[rt] >> sa
    /// - 64-bit:
    ///   - temp    <- GPR[rt] >> sa
    ///   - GPR[rd] <- sign_extend_u64::<32>(temp)
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_srl() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b000010;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rs_in: u64 = 0x11111111_11111111; // unused for SRL
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // Regular shift
        let sa = 15;
        let rt_in: u64 = 0x00000000_81234567;
        let rd_out: u64 = ((rt_in >> sa) as u32) as u64;

        test_rtype_instr(
            "SRL", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );

        // Shift by 0 (sign extend in 64-bit mode)
        let sa = 0;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out: u64 = (rt_in as i32) as u64;

        test_rtype_instr(
            "SRL", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the SRLV instruction.
    ///
    /// # SRLV:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rd] <- GPR[rt] >> (GPR[rs] & 31)
    /// - 64-bit:
    ///   - temp    <- GPR[rt] >> (GPR[rs] & 31)
    ///   - GPR[rd] <- sign_extend_u64::<32>(temp)
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_srlv() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b000110;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let sa: u32 = 0; // unused for SRLV
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // Regular shift
        let rs_in: u64 = 0xFFFFFFFF_FFFFFFFF;
        let rt_in: u64 = 0x00000000_81234567;
        let rd_out: u64 = (((rt_in as u32) >> (rs_in & 31)) as i32) as u64;

        test_rtype_instr(
            "SRLV", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );

        // Shift by 0 (sign extend in 64-bit mode)
        let rs_in: u64 = 0x00000000_00000000;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out: u64 = (rt_in as i32) as u64;

        test_rtype_instr(
            "SRLV", OP, rs, rt, rd, sa, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
        );
    }

    /// Test the DSRL instruction.
    ///
    /// # DSRL:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - GPR[rd] <- GPR[rt] >> sa
    /// ## Exceptions:
    /// - Reserved Instruction
    #[test]
    fn test_dsrl() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b111011;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rs_in: u64 = 0x11111111_11111111; // unused for DSRL
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DSRL",
            OP,
            rs,
            rt,
            rd,
            0u32,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // Regular shift
        let sa = 15;
        let rt_in: u64 = 0x00000000_81234567;
        let rd_out: u64 = rt_in >> sa;

        test_rtype_instr(
            "DSRL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );

        // Shift by 0 (NOP in 64-bit mode)
        let sa = 0;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out: u64 = rt_in;

        test_rtype_instr(
            "DSRL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );
    }

    /// Test the DSRLV instruction.
    ///
    /// # DSRLV:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - GPR[rd] <- GPR[rt] >> (GPR[rs] & 31)
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_dsrlv() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b010111;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let sa: u32 = 0; // unused for DSRLV
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DSRLV",
            OP,
            rs,
            rt,
            rd,
            0u32,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // Regular shift
        let rs_in: u64 = 0xFFFFFFFF_FFFFFFFF;
        let rt_in: u64 = 0x00000000_81234567;
        let rd_out: u64 = rt_in >> (rs_in & 63);

        test_rtype_instr(
            "DSRLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );

        // Shift by 0 (NOP in 64-bit mode)
        let rs_in: u64 = 0x00000000_00000000;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out: u64 = rt_in;

        test_rtype_instr(
            "DSRLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );
    }

    /// Test the DSRL32 instruction.
    ///
    /// # DSRL32:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - Reserved Instruction Exception
    /// - 64-bit:
    ///   - GPR[rd] <- GPR[rt] >> (sa + 32)
    /// ## Exceptions:
    /// - Reserved Instruction
    #[test]
    fn test_dsrl32() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b111111;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rs_in: u64 = 0x11111111_11111111; // unused for DSRL32
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // 32-bit mode should throw Reserved Instruction exception
        test_rtype_instr(
            "DSRL32",
            OP,
            rs,
            rt,
            rd,
            0u32,
            FUNC,
            0u64,
            0u64,
            rd_in,
            rd_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        // Regular shift
        let sa = 15;
        let rt_in: u64 = 0x00000000_81234567;
        let rd_out: u64 = rt_in >> (sa + 32);

        test_rtype_instr(
            "DSRL32",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out,
            None,
            Some(RegSize::Reg64),
        );
    }

    /// Test the SLT instruction.
    ///
    /// # SLT:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - `GPR[rd] <- (GPR[rs] < GPR[rt] ? 1 : 0)` (signed comparison)
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_slt() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b101010;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        let test = |rs_in: u64, rt_in: u64| {
            let rs_cmp: i64 = rs_in as i64;
            let rt_cmp: i64 = rt_in as i64;
            let rd_out: u64 = if rs_cmp < rt_cmp { 1 } else { 0 };

            test_rtype_instr(
                "SLT", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
            );
        };

        // Condition true
        test(
            0xFFFFFFFF_FFFFFFEE, // -18
            0x00000000_00000000, // 0
        );

        // Condition false
        test(
            0x00000000_00000000, // 0
            0xFFFFFFFF_FFFFFFEE, // -18
        );

        // Inputs equal (should produce 0)
        test(0xABCDEF01_23456789, 0xABCDEF01_23456789);
    }

    /// Test the SLTI instruction.
    ///
    /// # SLTI:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rd] <- (GPR[rs] < sign_extend_u32::<16>(imm) ? 1 : 0)` (signed comparison)
    /// - 64-bit:
    ///   - `GPR[rd] <- (GPR[rs] < sign_extend_u64::<16>(imm) ? 1 : 0)` (signed comparison)
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_slti() {
        const OP: u32 = 0b001010;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        let test = |rs_in: u64, imm: u16| {
            let rs_cmp: i64 = rs_in as i64;
            let imm_cmp: i64 = (imm as i16) as i64;
            let rt_out: u64 = if rs_cmp < imm_cmp { 1 } else { 0 };

            test_itype_instr("SLTI", OP, rs, rt, rs_in, rt_in, imm, rt_out, None, None);
        };

        // Condition true
        test(
            0xFFFFFFFF_FFFFFFEE, // GPR[rs] = -18
            0x0000,              // imm = 0
        );

        // Condition false
        test(
            0x00000000_00000000, // GPR[rs] = 0
            0xFFEE,              // imm = -18
        );

        // Inputs equal (should produce 0)
        test(0x00000000_00007F8A, 0x7F8A);
        test(0xFFFFFFFF_FFFFFF8A, 0xFF8A);
    }

    /// Test the SLTU instruction.
    ///
    /// # SLTU:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - `GPR[rd] <- (GPR[rs] < GPR[rt] ? 1 : 0)` (unsigned comparison)
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_sltu() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b101011;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        let test = |rs_in: u64, rt_in: u64| {
            let rd_out: u64 = if rs_in < rt_in { 1 } else { 0 };

            test_rtype_instr(
                "SLTU", OP, rs, rt, rd, SA, FUNC, rs_in, rt_in, rd_in, rd_out, None, None,
            );
        };

        // Condition true
        test(0x00000000_FF000000, 0xFFFFFFFF_FFFF8000);

        // Condition false
        test(0xFFFFFFFF_FFFFFFFF, 0x00000000_00000000);

        // Inputs equal (should produce 0)
        test(0x34343434_56789ABC, 0x34343434_56789ABC);
    }

    /// Test the SLTIU instruction.
    ///
    /// # SLTIU:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `GPR[rd] <- (GPR[rs] < sign_extend_u32::<16>(imm) ? 1 : 0)` (unsigned comparison)
    /// - 64-bit:
    ///   - `GPR[rd] <- (GPR[rs] < sign_extend_u64::<16>(imm) ? 1 : 0)` (unsigned comparison)
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_sltiu() {
        const OP: u32 = 0b001011;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        let test = |rs_in: u64, imm: u16| {
            let imm_cmp: u64 = imm as u64;
            let rt_out: u64 = if rs_in < imm_cmp { 1 } else { 0 };

            test_itype_instr("SLTIU", OP, rs, rt, rs_in, rt_in, imm, rt_out, None, None);
        };

        // Condition true
        test(0xFFFFFFFF_FF000000, 0x8000);

        // Condition false
        test(0x00000000_00000000, 0xFFEE);
        test(
            0x00000000_00000000, // GPR[rs] = 0
            0x0001,              // imm = 1
        );

        // Inputs equal (should produce 0)
        test(0xFFFFFFFF_FFFF8055, 0x8055);
    }

    /// Test the DIV instruction when `GPR[rt] != 0`.
    ///
    /// # DIV:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - `LO <- GPR[rs] / GPR[rt]` (signed division)
    ///   - `HI <- GPR[rs] % GPR[rt]`
    /// - 64-bit:
    ///   - `q <- GPR[rs] / GPR[rt]` (signed division)
    ///   - `r <- GPR[rs] % GPR[rt]`
    ///   - `LO <- sign_extend_u64::<32>(q)`
    ///   - `HI <- sign_extend_u64::<32>(r)`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_div_happy_path() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b011010;

        let rs = 1;
        let rt = 2;
        let hi_in = 0xAAAAAAAA_BBBBBBBB;
        let lo_in = 0xCCCCCCCC_DDDDDDDD;

        // Test 1:
        //   GPR[rs] = 599
        //   GPR[rt] = 144
        //   Expected LO = 599 / 144 = 4
        //   Expected HI = 599 % 144 = 23
        test_divmul_instr(
            "DIV - Happy Path 1",
            OP,
            rs,
            rt,
            FUNC,
            599,
            144,
            hi_in,
            lo_in,
            23,
            4,
            None,
            RegSize::Reg32,
        );

        test_divmul_instr(
            "DIV - Happy Path 1",
            OP,
            rs,
            rt,
            FUNC,
            599,
            144,
            hi_in,
            lo_in,
            23,
            4,
            None,
            RegSize::Reg64,
        );

        // Test 2:
        //   GPR[rs] = 78255
        //   GPR[rt] = -94723
        //   Expected LO = 78255 / -94723 = 0
        //   Expected HI = 78255 % -94723 = 78255
        test_divmul_instr(
            "DIV - Happy Path 2",
            OP,
            rs,
            rt,
            FUNC,
            78255,
            -94723i32 as u64,
            hi_in,
            lo_in,
            78255,
            0,
            None,
            RegSize::Reg32,
        );

        test_divmul_instr(
            "DIV - Happy Path 2",
            OP,
            rs,
            rt,
            FUNC,
            78255,
            -94723i32 as u64,
            hi_in,
            lo_in,
            78255,
            0,
            None,
            RegSize::Reg64,
        );

        // Test 2:
        //   GPR[rs] = 1859307798
        //   GPR[rt] = -19
        //   Expected LO = 1859307798 / -19 = -97858305
        //   Expected HI = 1859307798 % -19 = 3
        test_divmul_instr(
            "DIV - Happy Path 2",
            OP,
            rs,
            rt,
            FUNC,
            1859307798,
            -19i32 as u64,
            hi_in,
            lo_in,
            3,
            -97858305i32 as u64,
            None,
            RegSize::Reg32,
        );

        test_divmul_instr(
            "DIV - Happy Path 2",
            OP,
            rs,
            rt,
            FUNC,
            1859307798,
            -19i32 as u64,
            hi_in,
            lo_in,
            3,
            -97858305i32 as u64,
            None,
            RegSize::Reg64,
        );
    }

    /// Test the DIV instruction when `GPR[rt] == 0`.
    ///
    /// # DIV:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - `LO <- (GPR[rs] < 0 ? 1 : -1)`
    ///   - `HI <- sign_extend_u64::<32>(GPR[rs])`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_div_unhappy_path() {
        const OP: u32 = 0b000000;
        const FUNC: u32 = 0b011010;

        let rs = 1;
        let rt = 2;
        let hi_in = 0xAAAAAAAA_BBBBBBBB;
        let lo_in = 0xCCCCCCCC_DDDDDDDD;

        // Test 1:
        //   GPR[rs] = 55
        //   GPR[rt] = 0
        //   Expected HI = 55
        //   Expected LO = -1
        test_divmul_instr(
            "DIV - Unhappy Path 1",
            OP,
            rs,
            rt,
            FUNC,
            55,
            0,
            hi_in,
            lo_in,
            55,
            -1i32 as u64,
            None,
            RegSize::Reg32,
        );

        test_divmul_instr(
            "DIV - Unhappy Path 1",
            OP,
            rs,
            rt,
            FUNC,
            55,
            0,
            hi_in,
            lo_in,
            55,
            -1i32 as u64,
            None,
            RegSize::Reg64,
        );

        // Test 2:
        //   GPR[rs] = 0x00000000_80000000
        //   GPR[rt] = 0
        //   Expected LO = 0x00000000_00000001
        //   Expected HI = 0xFFFFFFFF_80000000
        test_divmul_instr(
            "DIV - Unhappy Path 1",
            OP,
            rs,
            rt,
            FUNC,
            0x00000000_80000000,
            0,
            hi_in,
            lo_in,
            0xFFFFFFFF_80000000,
            0x00000000_00000001,
            None,
            RegSize::Reg32,
        );

        test_divmul_instr(
            "DIV - Unhappy Path 1",
            OP,
            rs,
            rt,
            FUNC,
            0x00000000_80000000,
            0,
            hi_in,
            lo_in,
            0xFFFFFFFF_80000000,
            0x00000000_00000001,
            None,
            RegSize::Reg64,
        );
    }
}

mod load_store_instructions {
    use super::util::*;
    use crate::processors::vr4300::{CpuException, RegSize};

    /// Test the LB instruction.
    ///
    /// # LB:
    /// ## Type: I-Type
    /// ## Operation:
    /// - `GPR[rt] <- sign_extend_u64::<8>(Memory[GPR[rs] + sign_extend(imm)])`
    /// ## Exceptions:
    /// - None (any byte address is valid)
    #[test]
    fn test_lb() {
        const OP: u32 = 0b100000;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        let test = |byte: u8, expected: u64| {
            test_load_instr(
                "LB",
                OP,
                rs,
                rt,
                base,
                0,
                rt_in,
                base,
                &[byte],
                expected,
                None,
                None,
            );
        };

        test(0x7F, 0x00000000_0000007F); // positive, no sign extension
        test(0x80, 0xFFFFFFFF_FFFFFF80); // negative, sign-extended
        test(0xFF, 0xFFFFFFFF_FFFFFFFF);
    }

    /// Test the LBU instruction.
    ///
    /// # LBU:
    /// ## Type: I-Type
    /// ## Operation:
    /// - `GPR[rt] <- zero_extend_u64::<8>(Memory[GPR[rs] + sign_extend(imm)])`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_lbu() {
        const OP: u32 = 0b100100;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        let test = |byte: u8| {
            test_load_instr(
                "LBU",
                OP,
                rs,
                rt,
                base,
                0,
                rt_in,
                base,
                &[byte],
                byte as u64,
                None,
                None,
            );
        };

        test(0x7F);
        test(0x80);
        test(0xFF);
    }

    /// Test the LH instruction.
    ///
    /// # LH:
    /// ## Type: I-Type
    /// ## Operation:
    /// - `GPR[rt] <- sign_extend_u64::<16>(Memory[GPR[rs] + sign_extend(imm)])`
    /// ## Exceptions:
    /// - AddressErrorLoad if the effective address is not 2-byte aligned
    #[test]
    fn test_lh() {
        const OP: u32 = 0b100001;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        test_load_instr(
            "LH",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[0x12, 0x34],
            0x00000000_00001234,
            None,
            None,
        );
        test_load_instr(
            "LH",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[0x81, 0x23],
            0xFFFFFFFF_FFFF8123,
            None,
            None,
        );

        // Unaligned -> AddressErrorLoad, rt unchanged
        test_load_instr(
            "LH",
            OP,
            rs,
            rt,
            base,
            1,
            rt_in,
            base + 1,
            &[0x00, 0x00],
            rt_in,
            Some(CpuException::AddressErrorLoad),
            None,
        );
    }

    /// Test the LHU instruction.
    ///
    /// # LHU:
    /// ## Type: I-Type
    /// ## Operation:
    /// - `GPR[rt] <- zero_extend_u64::<16>(Memory[GPR[rs] + sign_extend(imm)])`
    /// ## Exceptions:
    /// - AddressErrorLoad if the effective address is not 2-byte aligned
    #[test]
    fn test_lhu() {
        const OP: u32 = 0b100101;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        test_load_instr(
            "LHU",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[0x12, 0x34],
            0x00000000_00001234,
            None,
            None,
        );
        test_load_instr(
            "LHU",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[0x81, 0x23],
            0x00000000_00008123,
            None,
            None,
        );

        test_load_instr(
            "LHU",
            OP,
            rs,
            rt,
            base,
            1,
            rt_in,
            base + 1,
            &[0x00, 0x00],
            rt_in,
            Some(CpuException::AddressErrorLoad),
            None,
        );
    }

    /// Test the LW instruction.
    ///
    /// # LW:
    /// ## Type: I-Type
    /// ## Operation:
    /// - `GPR[rt] <- sign_extend_u64::<32>(Memory[GPR[rs] + sign_extend(imm)])`
    /// ## Exceptions:
    /// - AddressErrorLoad if the effective address is not 4-byte aligned
    #[test]
    fn test_lw() {
        const OP: u32 = 0b100011;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        test_load_instr(
            "LW",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[0x12, 0x34, 0x56, 0x78],
            0x00000000_12345678,
            None,
            None,
        );
        test_load_instr(
            "LW",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[0x81, 0x23, 0x45, 0x67],
            0xFFFFFFFF_81234567,
            None,
            None,
        );

        for imm in 1..4u16 {
            test_load_instr(
                "LW",
                OP,
                rs,
                rt,
                base,
                imm,
                rt_in,
                base + imm as u64,
                &[0, 0, 0, 0],
                rt_in,
                Some(CpuException::AddressErrorLoad),
                None,
            );
        }
    }

    /// Test the LWU instruction.
    ///
    /// # LWU:
    /// ## Type: I-Type
    /// ## Operation:
    /// - `GPR[rt] <- Memory[GPR[rs] + sign_extend(imm)]`
    /// ## Exceptions:
    /// - AddressErrorLoad if the effective address is not 4-byte aligned
    #[test]
    fn test_lwu() {
        const OP: u32 = 0b100111;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        test_load_instr(
            "LW",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[0x12, 0x34, 0x56, 0x78],
            0x00000000_12345678,
            None,
            None,
        );
        test_load_instr(
            "LW",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[0x81, 0x23, 0x45, 0x67],
            0x00000000_81234567,
            None,
            None,
        );

        for imm in 1..4u16 {
            test_load_instr(
                "LW",
                OP,
                rs,
                rt,
                base,
                imm,
                rt_in,
                base + imm as u64,
                &[0, 0, 0, 0],
                rt_in,
                Some(CpuException::AddressErrorLoad),
                None,
            );
        }
    }

    /// Test the LD instruction.
    ///
    /// # LD:
    /// ## Type: I-Type
    /// ## Operation:
    /// - 32-bit: Reserved Instruction Exception
    /// - 64-bit: `GPR[rt] <- Memory[GPR[rs] + sign_extend(imm)]`
    /// ## Exceptions:
    /// - Reserved Instruction (32-bit mode)
    /// - AddressErrorLoad if the effective address is not 8-byte aligned
    #[test]
    fn test_ld() {
        const OP: u32 = 0b110111;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let bytes: [u8; 8] = [0x81, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];

        test_load_instr(
            "LD",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &bytes,
            rt_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        test_load_instr(
            "LD",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &bytes,
            0x81234567_89ABCDEF,
            None,
            Some(RegSize::Reg64),
        );

        for imm in 1..8u16 {
            test_load_instr(
                "LD",
                OP,
                rs,
                rt,
                base,
                imm,
                rt_in,
                base + imm as u64,
                &[0; 8],
                rt_in,
                Some(CpuException::AddressErrorLoad),
                Some(RegSize::Reg64),
            );
        }
    }

    /// Test the LL instruction.
    ///
    /// # LL:
    /// ## Type: I-Type
    /// ## Operation:
    /// - `GPR[rt] <- sign_extend_u64::<32>(Memory[GPR[rs] + sign_extend(imm)])`
    /// - `LLbit <- 1`
    /// ## Exceptions:
    /// - AddressErrorLoad if the effective address is not 4-byte aligned
    #[test]
    fn test_ll() {
        const OP: u32 = 0b110000;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        test_ll_instr(
            "LL",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            &[0x81, 0x23, 0x45, 0x67],
            0xFFFFFFFF_81234567,
            None,
            true,
            None,
        );

        for imm in 1..4u16 {
            test_ll_instr(
                "LL",
                OP,
                rs,
                rt,
                base,
                imm,
                rt_in,
                &[0, 0, 0, 0],
                rt_in,
                Some(CpuException::AddressErrorLoad),
                false,
                None,
            );
        }
    }

    /// Test the LLD instruction.
    ///
    /// # LLD:
    /// ## Type: I-Type
    /// ## Operation:
    /// - 32-bit: Reserved Instruction Exception
    /// - 64-bit: `GPR[rt] <- Memory[GPR[rs] + sign_extend(imm)]`, `LLbit <- 1`
    /// ## Exceptions:
    /// - Reserved Instruction (32-bit mode)
    /// - AddressErrorLoad if the effective address is not 8-byte aligned
    #[test]
    fn test_lld() {
        const OP: u32 = 0b110100;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let bytes: [u8; 8] = [0x81, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF];

        test_ll_instr(
            "LLD",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            &bytes,
            rt_in,
            Some(CpuException::ReservedInstruction),
            false,
            Some(RegSize::Reg32),
        );

        test_ll_instr(
            "LLD",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            &bytes,
            0x81234567_89ABCDEF,
            None,
            true,
            Some(RegSize::Reg64),
        );

        for imm in 1..8u16 {
            test_ll_instr(
                "LLD",
                OP,
                rs,
                rt,
                base,
                imm,
                rt_in,
                &[0; 8],
                rt_in,
                Some(CpuException::AddressErrorLoad),
                false,
                Some(RegSize::Reg64),
            );
        }
    }

    /// Test the LUI instruction.
    ///
    /// # LUI:
    /// ## Type: I-Type
    /// ## Operation:
    /// - `GPR[rt] <- sign_extend_u64::<32>(imm << 16)`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_lui() {
        const OP: u32 = 0b001111;
        let rs: u32 = 0; // unused by LUI
        let rt: u32 = 2;
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        let test = |imm: u16, expected: u64| {
            test_itype_instr("LUI", OP, rs, rt, 0, rt_in, imm, expected, None, None);
        };

        test(0x1234, 0x00000000_12340000);
        test(0x8000, 0xFFFFFFFF_80000000); // sign-extended
    }

    /// Test the LWL instruction.
    ///
    /// # LWL:
    /// ## Type: I-Type
    /// ## Operation:
    /// - Merges the high-order bytes of the addressed word into the
    ///   low-order bytes of `GPR[rt]`, then sign-extends the 32-bit result.
    /// ## Exceptions:
    /// - None (never faults on alignment)
    #[test]
    fn test_lwl() {
        const OP: u32 = 0b100010;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let word_bytes: [u8; 4] = [0x00, 0x11, 0x22, 0x33];
        let word = u32::from_be_bytes(word_bytes);
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        for byte in 0..4u32 {
            let shift = 8 * byte;
            let mask: u32 = 0xFFFFFFFFu32 << shift;
            let merged: u32 = ((rt_in as u32) & !mask) | (word << shift);
            let expected: u64 = (merged as i32) as i64 as u64;

            test_load_instr(
                "LWL",
                OP,
                rs,
                rt,
                base,
                byte as u16,
                rt_in,
                base,
                &word_bytes,
                expected,
                None,
                None,
            );
        }
    }

    /// Test the LWR instruction.
    ///
    /// # LWR:
    /// ## Type: I-Type
    /// ## Operation:
    /// - Merges the low-order bytes of the addressed word into the
    ///   high-order bytes of `GPR[rt]`, then sign-extends the 32-bit result.
    /// ## Exceptions:
    /// - None (never faults on alignment)
    #[test]
    fn test_lwr() {
        const OP: u32 = 0b100110;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let word_bytes: [u8; 4] = [0x11, 0x22, 0x33, 0x44];
        let word = u32::from_be_bytes(word_bytes);
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        for byte in 0..4u32 {
            let shift = 8 * (3 - byte);
            let mask: u32 = 0xFFFFFFFFu32 >> shift;
            let merged: u32 = ((rt_in as u32) & !mask) | (word >> shift);
            let expected: u64 = (merged as i32) as i64 as u64;

            test_load_instr(
                "LWR",
                OP,
                rs,
                rt,
                base,
                byte as u16,
                rt_in,
                base,
                &word_bytes,
                expected,
                None,
                None,
            );
        }
    }

    /// Test the LDL instruction.
    ///
    /// # LDL:
    /// ## Type: I-Type
    /// ## Operation:
    /// - 32-bit: Reserved Instruction Exception
    /// - 64-bit: merges the high-order bytes of the addressed doubleword
    ///   into the low-order bytes of `GPR[rt]`.
    /// ## Exceptions:
    /// - Reserved Instruction (32-bit mode); never faults on alignment
    #[test]
    fn test_ldl() {
        const OP: u32 = 0b011010;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let dword_bytes: [u8; 8] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
        let dword = u64::from_be_bytes(dword_bytes);
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        test_load_instr(
            "LDL",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &dword_bytes,
            rt_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        for byte in 0..8u64 {
            let shift = 8 * byte;
            let mask: u64 = u64::MAX << shift;
            let merged: u64 = (rt_in & !mask) | (dword << shift);

            test_load_instr(
                "LDL",
                OP,
                rs,
                rt,
                base,
                byte as u16,
                rt_in,
                base,
                &dword_bytes,
                merged,
                None,
                Some(RegSize::Reg64),
            );
        }
    }

    /// Test the LDR instruction.
    ///
    /// # LDR:
    /// ## Type: I-Type
    /// ## Operation:
    /// - 32-bit: Reserved Instruction Exception
    /// - 64-bit: merges the low-order bytes of the addressed doubleword
    ///   into the high-order bytes of `GPR[rt]`.
    /// ## Exceptions:
    /// - Reserved Instruction (32-bit mode); never faults on alignment
    #[test]
    fn test_ldr() {
        const OP: u32 = 0b011011;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let dword_bytes: [u8; 8] = [0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77];
        let dword = u64::from_be_bytes(dword_bytes);
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        test_load_instr(
            "LDR",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &dword_bytes,
            rt_in,
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        for byte in 0..8u64 {
            let shift = 8 * (7 - byte);
            let mask: u64 = u64::MAX >> shift;
            let merged: u64 = (rt_in & !mask) | (dword >> shift);

            test_load_instr(
                "LDR",
                OP,
                rs,
                rt,
                base,
                byte as u16,
                rt_in,
                base,
                &dword_bytes,
                merged,
                None,
                Some(RegSize::Reg64),
            );
        }
    }

    // -------------------------------------------------------------------
    // Stores
    // -------------------------------------------------------------------

    /// Test the SB instruction.
    ///
    /// # SB:
    /// ## Type: I-Type
    /// ## Operation:
    /// - `Memory[GPR[rs] + sign_extend(imm)] <- GPR[rt][7:0]`
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_sb() {
        const OP: u32 = 0b101000;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;

        test_store_instr(
            "SB",
            OP,
            rs,
            rt,
            base,
            0,
            0xAAAAAAAA_AAAAAA7F,
            base,
            &[0x7F],
            None,
            None,
        );
        test_store_instr(
            "SB",
            OP,
            rs,
            rt,
            base,
            0,
            0x00000000_000000FF,
            base,
            &[0xFF],
            None,
            None,
        );
    }

    /// Test the SH instruction.
    ///
    /// # SH:
    /// ## Type: I-Type
    /// ## Operation:
    /// - `Memory[GPR[rs] + sign_extend(imm)] <- GPR[rt][15:0]`
    /// ## Exceptions:
    /// - AddressErrorStore if the effective address is not 2-byte aligned
    #[test]
    fn test_sh() {
        const OP: u32 = 0b101001;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xAAAAAAAA_A1B2C3D4;

        test_store_instr(
            "SH",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[0xC3, 0xD4],
            None,
            None,
        );

        test_store_instr(
            "SH",
            OP,
            rs,
            rt,
            base,
            1,
            rt_in,
            base + 1,
            &[STORE_SENTINEL, STORE_SENTINEL],
            Some(CpuException::AddressErrorStore),
            None,
        );
    }

    /// Test the SW instruction.
    ///
    /// # SW:
    /// ## Type: I-Type
    /// ## Operation:
    /// - `Memory[GPR[rs] + sign_extend(imm)] <- GPR[rt][31:0]`
    /// ## Exceptions:
    /// - AddressErrorStore if the effective address is not 4-byte aligned
    #[test]
    fn test_sw() {
        const OP: u32 = 0b101011;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xAAAAAAAA_81234567;

        test_store_instr(
            "SW",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[0x81, 0x23, 0x45, 0x67],
            None,
            None,
        );

        for imm in 1..4u16 {
            test_store_instr(
                "SW",
                OP,
                rs,
                rt,
                base,
                imm,
                rt_in,
                base + imm as u64,
                &[STORE_SENTINEL; 4],
                Some(CpuException::AddressErrorStore),
                None,
            );
        }
    }

    /// Test the SD instruction.
    ///
    /// # SD:
    /// ## Type: I-Type
    /// ## Operation:
    /// - 32-bit: Reserved Instruction Exception
    /// - 64-bit: `Memory[GPR[rs] + sign_extend(imm)] <- GPR[rt]`
    /// ## Exceptions:
    /// - Reserved Instruction (32-bit mode)
    /// - AddressErrorStore if the effective address is not 8-byte aligned
    #[test]
    fn test_sd() {
        const OP: u32 = 0b111111;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0x81234567_89ABCDEF;

        test_store_instr(
            "SD",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[STORE_SENTINEL; 8],
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        test_store_instr(
            "SD",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[0x81, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF],
            None,
            Some(RegSize::Reg64),
        );

        for imm in 1..8u16 {
            test_store_instr(
                "SD",
                OP,
                rs,
                rt,
                base,
                imm,
                rt_in,
                base + imm as u64,
                &[STORE_SENTINEL; 8],
                Some(CpuException::AddressErrorStore),
                Some(RegSize::Reg64),
            );
        }
    }

    /// Test the SWL instruction.
    ///
    /// # SWL:
    /// ## Type: I-Type
    /// ## Operation:
    /// - Stores the high-order bytes of `GPR[rt][31:0]` into the low-order
    ///   bytes of the addressed word, preserving the rest of the word.
    /// ## Exceptions:
    /// - None (never faults on alignment)
    #[test]
    fn test_swl() {
        const OP: u32 = 0b101010;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xFFFFFFFF_AABBCCDD;

        test_store_instr(
            "SWL",
            OP,
            rs,
            rt,
            base,
            0x0000,
            rt_in,
            base,
            &0xAABBCCDDu32.to_be_bytes(),
            None,
            None,
        );

        test_store_instr(
            "SWL",
            OP,
            rs,
            rt,
            base,
            0x0001,
            rt_in,
            base,
            &0xEEAABBCCu32.to_be_bytes(),
            None,
            None,
        );

        test_store_instr(
            "SWL",
            OP,
            rs,
            rt,
            base,
            0x0002,
            rt_in,
            base,
            &0xEEEEAABBu32.to_be_bytes(),
            None,
            None,
        );

        test_store_instr(
            "SWL",
            OP,
            rs,
            rt,
            base,
            0x0003,
            rt_in,
            base,
            &0xEEEEEEAAu32.to_be_bytes(),
            None,
            None,
        );
    }

    /// Test the SWR instruction.
    ///
    /// # SWR:
    /// ## Type: I-Type
    /// ## Operation:
    /// - Stores the low-order bytes of `GPR[rt][31:0]` into the high-order
    ///   bytes of the addressed word, preserving the rest of the word.
    /// ## Exceptions:
    /// - None (never faults on alignment)
    #[test]
    fn test_swr() {
        const OP: u32 = 0b101110;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0xFFFFFFFF_AABBCCDD;

        test_store_instr(
            "SWR",
            OP,
            rs,
            rt,
            base,
            0x0000,
            rt_in,
            base,
            &0xDDEEEEEEu32.to_be_bytes(),
            None,
            None,
        );

        test_store_instr(
            "SWR",
            OP,
            rs,
            rt,
            base,
            0x0001,
            rt_in,
            base,
            &0xCCDDEEEEu32.to_be_bytes(),
            None,
            None,
        );

        test_store_instr(
            "SWR",
            OP,
            rs,
            rt,
            base,
            0x0002,
            rt_in,
            base,
            &0xBBCCDDEEu32.to_be_bytes(),
            None,
            None,
        );

        test_store_instr(
            "SWR",
            OP,
            rs,
            rt,
            base,
            0x0003,
            rt_in,
            base,
            &0xAABBCCDDu32.to_be_bytes(),
            None,
            None,
        );
    }

    /// Test the SDL instruction.
    ///
    /// # SDL:
    /// ## Type: I-Type
    /// ## Operation:
    /// - 32-bit: Reserved Instruction Exception
    /// - 64-bit: stores the high-order bytes of `GPR[rt]` into the
    ///   low-order bytes of the addressed doubleword.
    /// ## Exceptions:
    /// - Reserved Instruction (32-bit mode); never faults on alignment
    #[test]
    fn test_sdl() {
        const OP: u32 = 0b101100;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0x11223344_55667788;
        let mem_in: u64 = u64::from_be_bytes([STORE_SENTINEL; 8]);

        test_store_instr(
            "SDL",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &mem_in.to_be_bytes(),
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        for i in 0..8u64 {
            let shift = 8 * i;
            let inv_mask: u64 = u64::MAX >> shift;
            let expected = (mem_in & !inv_mask) | ((rt_in >> shift) & inv_mask);

            test_store_instr(
                "SDL",
                OP,
                rs,
                rt,
                base,
                i as u16,
                rt_in,
                base,
                &expected.to_be_bytes(),
                None,
                Some(RegSize::Reg64),
            );
        }
    }

    /// Test the SDR instruction.
    ///
    /// # SDR:
    /// ## Type: I-Type
    /// ## Operation:
    /// - 32-bit: Reserved Instruction Exception
    /// - 64-bit: stores the low-order bytes of `GPR[rt]` into the
    ///   high-order bytes of the addressed doubleword.
    /// ## Exceptions:
    /// - Reserved Instruction (32-bit mode); never faults on alignment
    #[test]
    fn test_sdr() {
        const OP: u32 = 0b101101;
        let rs: u32 = 1;
        let rt: u32 = 2;
        let base = RDRAM_BASE + 0x100;
        let rt_in: u64 = 0x11223344_55667788;
        let old_dword = u64::from_be_bytes([STORE_SENTINEL; 8]);

        test_store_instr(
            "SDR",
            OP,
            rs,
            rt,
            base,
            0,
            rt_in,
            base,
            &[STORE_SENTINEL; 8],
            Some(CpuException::ReservedInstruction),
            Some(RegSize::Reg32),
        );

        for byte in 0..8u64 {
            let shift = 8 * (7 - byte);
            let inv_mask: u64 = u64::MAX << shift;
            let merged: u64 = (old_dword & !inv_mask) | ((rt_in << shift) & inv_mask);

            test_store_instr(
                "SDR",
                OP,
                rs,
                rt,
                base,
                byte as u16,
                rt_in,
                base,
                &merged.to_be_bytes(),
                None,
                Some(RegSize::Reg64),
            );
        }
    }
}

mod branch_instructions {}

mod cop0_instructions {}

mod cop1_instructions {}

mod pipeline {}

mod timing {}
