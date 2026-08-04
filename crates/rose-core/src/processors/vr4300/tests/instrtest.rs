//! -----------------------------------------------------------------------
//! instrtest.rs: Individual tests for VR4300 instructions.
//!
//! Contains tests for all VR4300 instructions. We try to test happy paths and
//! unhappy (exception-causing) paths, and any edge cases that may arise.
//!
//! Author(s): MrBubblezsz
//! -----------------------------------------------------------------------

mod instrtest {
    use crate::processors::vr4300::{CpuException, CpuVR4300, RegSize};

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

    /// Builds an I-type isntruction from its components. See ITypeInstruciton
    /// struct for the layout diagram.
    fn itype_instr(op: u32, rs: u32, rt: u32, immediate: u32) -> u32 {
        (op << 26) | (rs << 21) | (rt << 16) | immediate
    }

    /// Builds an R-type isntruction from its components. See RTypeInstruciton
    /// struct for the layout diagram.
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
        cpu.regs[rs as usize] = rs_in;
        cpu.regs[rt as usize] = rt_in;
        cpu.execute_instruction(instr);

        let rt_out = cpu.regs[rt as usize];

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
        cpu.regs[rs as usize] = rs_in;
        cpu.regs[rt as usize] = rt_in;
        cpu.regs[rd as usize] = rd_in;
        cpu.execute_instruction(instr);

        let rd_out = cpu.regs[rd as usize];

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
    /// - 32-bit, 64-bit:
    ///   - GPR[rd] <- GPR[rs] + GPR[rt]
    /// - Mode determines bits affected.
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
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;
        // Should not affect high 32-bits
        let rd_out_32: u64 = (rd_in & 0xFFFFFFFF_00000000) | (rs_in & rt_in & 0x00000000_FFFFFFFF);
        // Affects full 64-bit register
        let rd_out_64: u64 = rs_in & rt_in;

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
            rd_out_32,
            None,
            RegSize::Reg32,
        );

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
            rd_out_64,
            None,
            RegSize::Reg64,
        );
    }

    /// Test the ANDI instruction.
    ///
    /// # ANDI:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - GPR[rt] <- zero_extend::<16>(imm) || GPR[rs]
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_andi() {
        const OP: u32 = 0b001100;

        let rs: u32 = 4;
        let rt: u32 = 5;

        let rs_in: u64 = 0xABCDEF01_23456789;
        let rt_in: u64 = 0xAAAAAAAA_AAAAAAAA;
        let immediate: u16 = 0xFEED;
        // Should not affect high 32-bits
        let rt_out_32: u64 =
            (rt_in & 0xFFFFFFFF_00000000) | (rs_in & immediate as u64 & 0x00000000_FFFFFFFF);
        // Affects full 64-bit register
        let rt_out_64: u64 = rs_in & immediate as u64;

        test_itype_instr(
            "ANDI",
            OP,
            rs,
            rt,
            rs_in,
            rt_in,
            immediate,
            rt_out_32,
            None,
            RegSize::Reg32,
        );

        test_itype_instr(
            "ANDI",
            OP,
            rs,
            rt,
            rs_in,
            rt_in,
            immediate,
            rt_out_64,
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
    /// - 32-bit, 64-bit:
    ///   - GPR[rd] <- GPR[rs] NOR GPR[rt]
    /// - Mode determines bits affected.
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
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;
        // Should not affect high 32-bits
        let rd_out_32: u64 =
            (rd_in & 0xFFFFFFFF_00000000) | ((!(rs_in | rt_in)) & 0x00000000_FFFFFFFF);
        // Affects full 64-bit register
        let rd_out_64: u64 = !(rs_in | rt_in);

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
            rd_out_32,
            None,
            RegSize::Reg32,
        );

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
    /// - 32-bit, 64-bit:
    ///   - GPR[rd] <- GPR[rs] + GPR[rt]
    /// - Mode determines bits affected.
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
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;
        // Should not affect high 32-bits
        let rd_out_32: u64 =
            (rd_in & 0xFFFFFFFF_00000000) | ((rs_in | rt_in) & 0x00000000_FFFFFFFF);
        // Affects full 64-bit register
        let rd_out_64: u64 = rs_in | rt_in;

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
            rd_out_32,
            None,
            RegSize::Reg32,
        );

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
            rd_out_64,
            None,
            RegSize::Reg64,
        );
    }

    /// Test the ANDI instruction.
    ///
    /// # ANDI:
    /// ## Type:
    /// - I-Type
    /// ## Operation:
    /// - 32-bit, 64-bit:
    ///   - GPR[rt] <- zero_extend::<16>(imm) || GPR[rs]
    /// ## Exceptions:
    /// - None
    #[test]
    fn test_ori() {
        const OP: u32 = 0b001101;

        let rs: u32 = 4;
        let rt: u32 = 5;

        let rs_in: u64 = 0xABCDEF01_23456789;
        let rt_in: u64 = 0xAAAAAAAA_AAAAAAAA;
        let immediate: u16 = 0xFEED;
        // Should not affect high 32-bits
        let rt_out_32: u64 =
            (rt_in & 0xFFFFFFFF_00000000) | ((rs_in | immediate as u64) & 0x00000000_FFFFFFFF);
        // Affects full 64-bit register
        let rt_out_64: u64 = rs_in | immediate as u64;

        test_itype_instr(
            "ORI",
            OP,
            rs,
            rt,
            rs_in,
            rt_in,
            immediate,
            rt_out_32,
            None,
            RegSize::Reg32,
        );

        test_itype_instr(
            "ORI",
            OP,
            rs,
            rt,
            rs_in,
            rt_in,
            immediate,
            rt_out_64,
            None,
            RegSize::Reg64,
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
    ///   - GPR[rd] <- GPR[rs] + GPR[rt]
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
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

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
            RegSize::Reg32,
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
            RegSize::Reg64,
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
            Some(CpuException::IntegerOverflow),
            RegSize::Reg64,
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
    ///   - GPR[rt] <- GPR[rs] + sign_extend_u64::<16>(imm)
    /// ## Exceptions:
    /// - Integer Overflow
    /// - Reserved Instruction
    #[test]
    fn test_daddi() {
        const OP: u32 = 0b011000;

        let rs: u32 = 4;
        let rt: u32 = 5;
        let rt_in: u64 = 0xAAAAAAAA_AAAAAAAA;

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
            RegSize::Reg32,
        );

        // No overflow
        let immediate: u16 = 0x0ACE;

        let rs_in: u64 = 0x0ACE0987_DEFACE00;
        let rt_out: u64 = rs_in + sign_extend_u64::<16>(immediate as u64);

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
            RegSize::Reg64,
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
            Some(CpuException::IntegerOverflow),
            RegSize::Reg64,
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
    ///   - GPR[rd] <- GPR[rs] + GPR[rt]
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
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

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
            RegSize::Reg32,
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
            RegSize::Reg64,
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
            RegSize::Reg64,
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
    ///   - GPR[rt] <- GPR[rs] + sign_extend_u64::<16>(imm)
    /// ## Exceptions:
    /// - Reserved Instruction
    #[test]
    fn test_daddiu() {
        const OP: u32 = 0b011001;

        let rs: u32 = 4;
        let rt: u32 = 5;
        let rt_in: u64 = 0xAAAAAAAA_AAAAAAAA;

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
            RegSize::Reg32,
        );

        // No overflow
        let immediate: u16 = 0x0ACE;

        let rs_in: u64 = 0x0ACE0987_DEFACE00;
        let rt_out: u64 = rs_in + sign_extend_u64::<16>(immediate as u64);

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
            RegSize::Reg64,
        );

        // Overflow test (output register unchanged)
        let immediate: u16 = 0xFFFF;

        let rs_in: u64 = 0x80000000_00000000;
        let rt_out: u64 = rs_in.wrapping_add(sign_extend_u64::<16>(immediate as u64));

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
            RegSize::Reg64,
        );
    }

    /// Test the SUB instruction.
    ///
    /// # SUB:
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
    fn test_sub() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100010;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

        // No overflow
        let rs_in_32: i32 = 99;
        let rt_in_32: i32 = 10;
        let rd_out_32: i32 = rs_in_32 - rt_in_32;

        let rs_in_64: i32 = 5002;
        let rt_in_64: i32 = 963;
        let rd_out_64: u64 = sign_extend_u64::<32>((rs_in_64 - rt_in_64) as u64);

        test_rtype_instr(
            "SUB",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            (rs_in_32 as i64) as u64,
            (rt_in_32 as i64) as u64,
            rd_in,
            (rd_out_32 as u32) as u64,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SUB",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            (rs_in_64 as i64) as u64,
            (rt_in_64 as i64) as u64,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );

        // Overflow test (output register unchanged)
        let rs_in_32: i32 = i32::MIN;
        let rt_in_32: i32 = 1;

        let rs_in_64: i32 = i32::MIN;
        let rt_in_64: i32 = 1;

        test_rtype_instr(
            "SUB",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            (rs_in_32 as i64) as u64,
            (rt_in_32 as i64) as u64,
            rd_in,
            rd_in,
            Some(CpuException::IntegerOverflow),
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SUB",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            (rs_in_64 as i64) as u64,
            (rt_in_64 as i64) as u64,
            rd_in,
            rd_in,
            Some(CpuException::IntegerOverflow),
            RegSize::Reg64,
        );
    }

    /// Test the SUBU instruction.
    ///
    /// # SUBU:
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
    fn test_subu() {
        const OP: u32 = 0b000000;
        const SA: u32 = 0b00000;
        const FUNC: u32 = 0b100011;

        let rs: u32 = 1;
        let rt: u32 = 2;
        let rd: u32 = 3;
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

        // No overflow
        let rs_in_32: i32 = 99;
        let rt_in_32: i32 = 10;
        let rd_out_32: i32 = rs_in_32 - rt_in_32;

        let rs_in_64: i32 = 5002;
        let rt_in_64: i32 = 963;
        let rd_out_64: u64 = sign_extend_u64::<32>((rs_in_64 - rt_in_64) as u64);

        test_rtype_instr(
            "SUBU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            (rs_in_32 as i64) as u64,
            (rt_in_32 as i64) as u64,
            rd_in,
            rd_out_32 as u64,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SUBU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            (rs_in_64 as i64) as u64,
            (rt_in_64 as i64) as u64,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );

        // Overflow test (no exception occurs)
        let rs_in_32: i32 = i32::MIN;
        let rt_in_32: i32 = 1;
        let rd_out_32: i32 = rs_in_32.wrapping_sub(rt_in_32);

        let rs_in_64: i32 = i32::MIN;
        let rt_in_64: i32 = 1;
        let rd_out_64: u64 = sign_extend_u64::<32>(rs_in_64.wrapping_sub(rt_in_64) as u64);

        test_rtype_instr(
            "SUBU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            (rs_in_32 as i64) as u64,
            (rt_in_32 as i64) as u64,
            rd_in,
            (rd_out_32 as u32) as u64,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SUBU",
            OP,
            rs,
            rt,
            rd,
            SA,
            FUNC,
            (rs_in_64 as i64) as u64,
            (rt_in_64 as i64) as u64,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
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
    ///   - GPR[rd] <- GPR[rs] - GPR[rt]
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
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

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
            RegSize::Reg32,
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
            RegSize::Reg64,
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
            Some(CpuException::IntegerOverflow),
            RegSize::Reg64,
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
    ///   - GPR[rd] <- GPR[rs] - GPR[rt]
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
        let rd_in: u64 = 0xAAAAAAAA_AAAAAAAA;

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
            RegSize::Reg32,
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
            RegSize::Reg64,
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
            RegSize::Reg64,
        );
    }
}
