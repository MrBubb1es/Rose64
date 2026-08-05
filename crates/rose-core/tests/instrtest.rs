//! -----------------------------------------------------------------------
//! instrtest.rs: Individual tests for VR4300 instructions.
//!
//! Contains tests for all VR4300 instructions. We try to test happy paths and
//! unhappy (exception-causing) paths, and any edge cases that may arise.
//!
//! Author(s): MrBubblezsz
//! -----------------------------------------------------------------------

mod instrtest {
    use rose_core::processors::vr4300::{CpuException, CpuVR4300, RegSize};

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

    const fn sign_extend_u32<const WIDTH: usize>(value: u32) -> u32 {
        let shift = 32 - WIDTH;
        (((value as i32) << shift) >> shift) as u32
    }

    const fn sign_extend_u64<const WIDTH: usize>(value: u64) -> u64 {
        let shift = 64 - WIDTH;
        (((value as i64) << shift) >> shift) as u64
    }

    /// Builds an I-type isntruction from its components. An I-type instruction
    /// is layed out as follows:
    ///  31    26 25  21 20  16 15                   0
    ///  ---------------------------------------------
    /// |  Op   |  rs  |  rt  |      immediate       |
    /// ---------------------------------------------
    fn itype_instr(op: u32, rs: u32, rt: u32, immediate: u32) -> u32 {
        (op << 26) | (rs << 21) | (rt << 16) | immediate
    }

    /// Builds an R-type isntruction from its components. An I-type instruction
    /// is layed out as follows:
    ///  31    26 25  21 20  16 15  11 10  6  5     0
    ///  ---------------------------------------------
    /// |  Op   |  rs  |  rt  |  rd  |  sa  |  func. |
    /// ---------------------------------------------
    fn rtype_instr(op: u32, rs: u32, rt: u32, rd: u32, sa: u32, func: u32) -> u32 {
        (op << 26) | (rs << 21) | (rt << 16) | (rd << 11) | (sa << 6) | func
    }

    /// Test the execution of an I-Type instruction. Checks register and
    /// exception output vs. expected.
    fn test_itype_instr(
        name: &str,
        op: u32,
        rs: u32,
        rt: u32,
        rs_in: u64,
        rt_in: u64,
        immediate: u16,
        expected_rt_out: u64,
        expected_exception: Option<CpuException>,
        reg_size: RegSize,
    ) {
        let mut cpu = CpuVR4300::new();
        let instr = itype_instr(op, rs, rt, immediate as u32);

        cpu.reg_size = reg_size;
        cpu.gpr[rs as usize] = rs_in;
        cpu.gpr[rt as usize] = rt_in;
        cpu.execute_instruction(instr);

        let rt_out = cpu.gpr[rt as usize];

        assert_eq!(
            cpu.last_exception,
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
            cpu.last_exception
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
            cpu.last_exception
        );
    }

    /// Test the execution of an R-Type instruction. Checks register and
    /// exception output vs. expected.
    fn test_rtype_instr(
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
        reg_size: RegSize,
    ) {
        let mut cpu = CpuVR4300::new();
        let instr = rtype_instr(op, rs, rt, rd, sa, func);

        cpu.reg_size = reg_size;
        cpu.gpr[rs as usize] = rs_in;
        cpu.gpr[rt as usize] = rt_in;
        cpu.gpr[rd as usize] = rd_in;
        cpu.execute_instruction(instr);

        let rd_out = cpu.gpr[rd as usize];

        assert_eq!(
            cpu.last_exception,
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
            cpu.last_exception
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
            cpu.last_exception
        );
    }

    /// Test the ADD instruction.
    ///
    /// # ADD:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rd] <- GPR[rs] + GPR[rt]
    /// - 64-bit:
    ///   - temp    <- GPR[rs] + GPR[rt]
    ///   - GPR[rd] <- sign_extend_u64::<32>(temp)
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
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

        // No overflow
        let rs_in_32: u32 = 0x1001FEDC;
        let rt_in_32: u32 = 0x81234567;
        let rd_out_32: u32 = rs_in_32 + rt_in_32;

        let rs_in_64: u64 = 0x00000000_1001FEDC;
        let rt_in_64: u64 = 0xFFFFFFFF_81234567;
        let rd_out_64: u64 = sign_extend_u64::<32>(rs_in_64 + rt_in_64);

        test_rtype_instr(
            "ADD",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in_32 as u64,
            rt_in_32 as u64,
            rd_in,
            rd_out_32 as u64,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "ADD",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in_64,
            rt_in_64,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );

        // Overflow test (output register unchanged)
        let rs_in_32: u32 = 0xFFFFFFFF;
        let rt_in_32: u32 = 0x80000000;
        let rd_out_32: u64 = rd_in;

        let rs_in_64: u64 = 0xFFFFFFFF_FFFFFFFF;
        let rt_in_64: u64 = 0xFFFFFFFF_80000000;
        let rd_out_64: u64 = rd_in;

        test_rtype_instr(
            "ADD",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in_32 as u64,
            rt_in_32 as u64,
            rd_in,
            rd_out_32,
            Some(CpuException::IntegerOverflow),
            RegSize::Reg32,
        );

        test_rtype_instr(
            "ADD",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in_64,
            rt_in_64,
            rd_in,
            rd_out_64,
            Some(CpuException::IntegerOverflow),
            RegSize::Reg64,
        );
    }

    /// Test the ADDI instruction.
    ///
    /// # ADDI:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rt] <- GRP[rs] + sign_extend_u32::<16>(immediate)
    /// - 64-bit:
    ///   - temp    <- GPR[rs] + sign_extend_u64::<16>(immediate)
    ///   - GPR[rt] <- sign_extend_u64::<32>(temp)
    /// ## Exceptions:
    /// - Integer Overflow
    #[test]
    fn test_addi() {
        const OP: u32 = 0b001000;

        let rs: u32 = 4;
        let rt: u32 = 5;
        let rt_in: u64 = 0xAAAAAAAA_AAAAAAAA;

        // No overflow
        let immediate: u16 = 0x0ACE;

        let rs_in_32: u32 = 0x0ACE0987;
        let rt_out_32: u32 = sign_extend_u32::<16>(immediate as u32) + rs_in_32;

        let rs_in_64: u64 = 0x00000000_0ACE0987;
        let rt_out_64: u64 =
            sign_extend_u64::<32>(sign_extend_u64::<16>(immediate as u64) + rs_in_64);

        test_itype_instr(
            "ADDI",
            OP,
            rs,
            rt,
            rs_in_32 as u64,
            rt_in,
            immediate,
            rt_out_32 as u64,
            None,
            RegSize::Reg32,
        );

        test_itype_instr(
            "ADDI",
            OP,
            rs,
            rt,
            rs_in_64,
            rt_in,
            immediate,
            rt_out_64,
            None,
            RegSize::Reg64,
        );

        // Overflow test (output register unchanged)
        let immediate: u16 = 0x7FFF;

        let rs_in_32: u32 = 0x7FFFFFFF;
        let rt_out_32: u64 = rt_in;

        let rs_in_64: u64 = 0x00000000_7FFFFFFF;
        let rt_out_64: u64 = rt_in;

        test_itype_instr(
            "ADDI",
            OP,
            rs,
            rt,
            rs_in_32 as u64,
            rt_in,
            immediate,
            rt_out_32,
            Some(CpuException::IntegerOverflow),
            RegSize::Reg32,
        );

        test_itype_instr(
            "ADDI",
            OP,
            rs,
            rt,
            rs_in_64,
            rt_in,
            immediate,
            rt_out_64,
            Some(CpuException::IntegerOverflow),
            RegSize::Reg64,
        );
    }

    /// Test the ADDIU instruction.
    ///
    /// # ADDIU:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rt] <- GRP[rs] + sign_extend_u32::<16>(immediate)
    /// - 64-bit:
    ///   - temp    <- GPR[rs] + sign_extend_u64::<16>(immediate)
    ///   - GPR[rt] <- sign_extend_u64::<32>(temp)
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_addiu() {
        const OP: u32 = 0b001001;

        let rs: u32 = 4;
        let rt: u32 = 5;
        let rt_in: u64 = 0xAAAAAAAA_AAAAAAAA;

        // No overflow
        let immediate: u16 = 0x0ACE;

        let rs_in_32: u32 = 0x0ACE0987;
        let rt_out_32: u32 = sign_extend_u32::<16>(immediate as u32) + rs_in_32;

        let rs_in_64: u64 = 0x00000000_0ACE0987;
        let rt_out_64: u64 =
            sign_extend_u64::<32>(sign_extend_u64::<16>(immediate as u64) + rs_in_64);

        test_itype_instr(
            "ADDIU",
            OP,
            rs,
            rt,
            rs_in_32 as u64,
            rt_in,
            immediate,
            rt_out_32 as u64,
            None,
            RegSize::Reg32,
        );

        test_itype_instr(
            "ADDIU",
            OP,
            rs,
            rt,
            rs_in_64,
            rt_in,
            immediate,
            rt_out_64,
            None,
            RegSize::Reg64,
        );

        // Overflow test
        let immediate: u16 = 0x7FFF;

        let rs_in_32: u32 = 0x7FFFFFFF;
        let rt_out_32: u32 = sign_extend_u32::<16>(immediate as u32).wrapping_add(rs_in_32);

        let rs_in_64: u64 = 0x00000000_7FFFFFFF;
        let rt_out_64: u64 =
            sign_extend_u64::<32>(sign_extend_u64::<16>(immediate as u64).wrapping_add(rs_in_64));

        test_itype_instr(
            "ADDI",
            OP,
            rs,
            rt,
            rs_in_32 as u64,
            rt_in,
            immediate,
            rt_out_32 as u64,
            None,
            RegSize::Reg32,
        );

        test_itype_instr(
            "ADDI",
            OP,
            rs,
            rt,
            rs_in_64,
            rt_in,
            immediate,
            rt_out_64,
            None,
            RegSize::Reg64,
        );
    }

    /// Test the ADDU instruction.
    ///
    /// # ADDU:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rd] <- GPR[rs] + GPR[rt]
    /// - 64-bit:
    ///   - temp    <- GPR[rs] + GPR[rt]
    ///   - GPR[rd] <- sign_extend_u64::<32>(temp)
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
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

        // No overflow
        let rs_in_32: u32 = 0x1001FEDC;
        let rt_in_32: u32 = 0x81234567;
        let rd_out_32: u32 = rs_in_32 + rt_in_32;

        let rs_in_64: u64 = 0x00000000_1001FEDC;
        let rt_in_64: u64 = 0xFFFFFFFF_81234567;
        let rd_out_64: u64 = sign_extend_u64::<32>(rs_in_64 + rt_in_64);

        test_rtype_instr(
            "ADDU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in_32 as u64,
            rt_in_32 as u64,
            rd_in,
            rd_out_32 as u64,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "ADDU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in_64,
            rt_in_64,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );

        // Overflow test
        let rs_in_32: u32 = 0xFFFFFFFF;
        let rt_in_32: u32 = 0x80000000;
        let rd_out_32: u32 = rs_in_32.wrapping_add(rt_in_32);

        let rs_in_64: u64 = 0xFFFFFFFF_FFFFFFFF;
        let rt_in_64: u64 = 0xFFFFFFFF_80000000;
        let rd_out_64: u64 = sign_extend_u64::<32>(rs_in_64.wrapping_add(rt_in_64));

        test_rtype_instr(
            "ADDU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in_32 as u64,
            rt_in_32 as u64,
            rd_in,
            rd_out_32 as u64,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "ADDU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            rs_in_64,
            rt_in_64,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );
    }

    /// Test the AND instruction.
    ///
    /// # AND:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rd] <- GPR[rs] & GPR[rt]
    /// - 64-bit:
    ///   - GPR[rd] <- GPR[rs] & GPR[rt]
    /// ## Exceptions:
    /// - None
    ///
    /// We only test the 64-bit version because the 32-bit version is identical.
    #[test]
    fn test_and() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100100;

        let rs = 1;
        let rt = 2;
        let rd = 3;
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

        let rs_in: u64 = 0xAAFF00AA_ABCDFFFF;
        let rt_in: u64 = 0x550FF0FA_FFFF1234;
        let rd_out: u64 = 0x000F00AA_ABCD1234;

        test_rtype_instr(
            "AND",
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
            RegSize::Reg64,
        );
    }

    /// Test the OR instruction.
    ///
    /// # OR:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rd] <- GPR[rs] | GPR[rt]
    /// - 64-bit:
    ///   - GPR[rd] <- GPR[rs] | GPR[rt]
    /// ## Exceptions:
    /// - None
    ///
    /// We only test the 64-bit version because the 32-bit version is identical.
    #[test]
    fn test_or() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100101;

        let rs = 1;
        let rt = 2;
        let rd = 3;
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

        let rs_in: u64 = 0xFFFF0000_11114444;
        let rt_in: u64 = 0xF0F0F0F0_22228888;
        let rd_out: u64 = 0xFFFFF0F0_3333CCCC;

        test_rtype_instr(
            "OR",
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
            RegSize::Reg64,
        );
    }

    /// Test the NOR instruction.
    ///
    /// # NOR:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rd] <- !(GPR[rs] | GPR[rt])
    /// - 64-bit:
    ///   - GPR[rd] <- !(GPR[rs] | GPR[rt])
    /// ## Exceptions:
    /// - None
    ///
    /// We only test the 64-bit version because the 32-bit version is identical.
    #[test]
    fn test_nor() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100111;

        let rs = 1;
        let rt = 2;
        let rd = 3;
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

        let rs_in: u64 = 0xFFFF0000_11114444;
        let rt_in: u64 = 0xF0F0F0F0_22228888;
        let rd_out: u64 = 0x00000F0F_CCCC3333;

        test_rtype_instr(
            "NOR",
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
            RegSize::Reg64,
        );
    }

    /// Test the XOR instruction.
    ///
    /// # XOR:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rd] <- GPR[rs] ^ GPR[rt]
    /// - 64-bit:
    ///   - GPR[rd] <- GPR[rs] ^ GPR[rt]
    /// ## Exceptions:
    /// - None
    ///
    /// We only test the 64-bit version because the 32-bit version is identical.
    #[test]
    fn test_xor() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100110;

        let rs = 1;
        let rt = 2;
        let rd = 3;
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

        let rs_in: u64 = 0xFFFF0000_F5A01234;
        let rt_in: u64 = 0xF0F0F0F0_3A5A3666;
        let rd_out: u64 = 0x0F0FF0F0_CFFA2452;

        test_rtype_instr(
            "XOR",
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
            RegSize::Reg64,
        );
    }
}
