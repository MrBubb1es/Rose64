//! -----------------------------------------------------------------------
//! instrtest.rs: Individual tests for VR4300 instructions.
//!
//! Contains tests for all VR4300 instructions. We try to test happy paths and
//! unhappy (exception-causing) paths, and any edge cases that may arise.
//!
//! Author(s): MrBubblezsz, logocrazymon
//! -----------------------------------------------------------------------

mod instrtest {
    use crate::processors::vr4300::{CpuException, CpuVR4300, RegSize};

    const MASK32: u64 = 0x00000000_FFFFFFFF;

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
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let rs_in_32: u32 = 0x1001FEDC;
        let rt_in_32: u32 = 0x81234567;
        let rd_out_32: u64 = (rd_in & !MASK32) | (rs_in_32 + rt_in_32) as u64;

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
            rd_out_32,
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
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let immediate: u16 = 0x0ACE;

        let rs_in_32: u32 = 0x0ACE0987;
        let rt_out_32: u64 =
            (rt_in & !MASK32) | (sign_extend_u32::<16>(immediate as u32) + rs_in_32) as u64;

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
            rt_out_32,
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
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let immediate: u16 = 0x0ACE;

        let rs_in_32: u32 = 0x0ACE0987;
        let rt_out_32: u64 = (rt_in & !MASK32) | ((immediate as u32) + rs_in_32) as u64;

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
            rt_out_32,
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
        let rt_out_32: u64 = (rt_in & !MASK32) | ((immediate as u32).wrapping_add(rs_in_32)) as u64;

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
            rt_out_32,
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
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let rs_in_32: u32 = 0x1001FEDC;
        let rt_in_32: u32 = 0x81234567;
        let rd_out_32: u64 = (rd_in & !MASK32) | (rs_in_32 + rt_in_32) as u64;

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
            rd_out_32,
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
        let rd_out_32: u64 = (rd_in & !MASK32) | rs_in_32.wrapping_add(rt_in_32) as u64;

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
            rd_out_32,
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
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let rd_out_32: u64 = (rd_in & !MASK32) | (rs_in & rt_in & MASK32);
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
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let immediate: u16 = 0xFEED;
        let rt_out_32: u64 = (rt_in & !MASK32) | (rs_in & immediate as u64 & MASK32);
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
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let rd_out_32: u64 = (rd_in & !MASK32) | ((!(rs_in | rt_in)) & MASK32);
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
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let rd_out_32: u64 = (rd_in & !MASK32) | ((rs_in | rt_in) & MASK32);
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
        let rt_in: u64 = 0xAAAAAAAA_BBBBBBBB;
        let immediate: u16 = 0xFEED;
        let rt_out_32: u64 = (rt_in & !MASK32) | ((rs_in | immediate as u64) & MASK32);
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
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let rs_in_32: i32 = 99;
        let rt_in_32: i32 = 10;
        let rd_out_32: u64 = (rd_in & !MASK32) | ((rs_in_32 - rt_in_32) as u32) as u64;

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
            rd_out_32,
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
        let rd_in: u64 = 0xAAAAAAAA_BBBBBBBB;

        // No overflow
        let rs_in_32: i32 = 99;
        let rt_in_32: i32 = 10;
        let rd_out_32: u64 = (rd_in & !MASK32) | ((rs_in_32 - rt_in_32) as u32) as u64;

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
            rd_out_32,
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
        let rd_out_32: u64 = (rd_in & !MASK32) | (rs_in_32.wrapping_sub(rt_in_32) as u32) as u64;

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
            rd_out_32,
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

    /// Test the SLL instruction.
    ///
    /// # SLL:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rd] <- GPR[rt] << sa
    /// - 64-bit:
    ///   - temp    <- GPR[rt] << sa
    ///   - GPR[rd] <- sign_extend_u64::<32>(temp)
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
        let rt_in_32: u32 = 0x81234567;
        let rd_out_32: u64 = (rd_in & !MASK32) | (rt_in_32 << sa) as u64;

        let rt_in_64: u64 = 0xFFFFFFFF_81234567;
        let rd_out_64: u64 = sign_extend_u64::<32>((rt_in_64 & 0xFFFFFFFF) << sa);

        test_rtype_instr(
            "SLL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in_32 as u64,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SLL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in_64,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );

        // Shift by 0 (sign extend in 64-bit mode)
        let sa = 0;
        let rt_in_32: u32 = 0x80000000;
        let rd_out_32: u64 = (rd_in & !MASK32) | rt_in_32 as u64;

        let rt_in_64: u64 = 0x00000000_80000000;
        let rd_out_64: u64 = sign_extend_u64::<32>(rt_in_64);

        test_rtype_instr(
            "SLL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in_32 as u64,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SLL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in_64,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );
    }

    /// Test the SLLV instruction.
    ///
    /// # SLLV:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rd] <- GPR[rt] << (GPR[rs] & 31)
    /// - 64-bit:
    ///   - temp    <- GPR[rt] << (GPR[rs] & 31)
    ///   - GPR[rd] <- sign_extend_u64::<32>(temp)
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
        let rt_in_32: u32 = 0x81234567;
        let rd_out_32: u64 = (rd_in & !MASK32) | (rt_in_32 << (rs_in & 31)) as u64;

        let rt_in_64: u64 = 0xFFFFFFFF_81234567;
        let rd_out_64: u64 = sign_extend_u64::<32>((rt_in_64 & 0xFFFFFFFF) << (rs_in & 31));

        test_rtype_instr(
            "SLLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in_32 as u64,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SLLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in_64,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );

        // Shift by 0 (sign extend in 64-bit mode)
        let rs_in: u64 = 0x00000000_00000000;
        let rt_in_32: u32 = 0x80000000;
        let rd_out_32: u64 = (rd_in & !MASK32) | rt_in_32 as u64;

        let rt_in_64: u64 = 0x00000000_80000000;
        let rd_out_64: u64 = sign_extend_u64::<32>(rt_in_64);

        test_rtype_instr(
            "SLLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in_32 as u64,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SLLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in_64,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
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
    ///   - GPR[rd] <- GPR[rt] << sa
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
            RegSize::Reg32,
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
            RegSize::Reg64,
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
            RegSize::Reg32,
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
    ///   - GPR[rd] <- GPR[rt] << (GPR[rs] & 31)
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
            RegSize::Reg32,
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
            RegSize::Reg64,
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
            RegSize::Reg32,
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
        let rd_out_32: u64 = (rd_in & !MASK32) | ((rt_in >> sa) as u32) as u64;
        let rd_out_64: u64 = sign_extend_u64::<32>(rd_out_32);

        test_rtype_instr(
            "SRA",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SRA",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );

        // Shift by 0 (NOP in 32-bit mode, sign extend in 64-bit mode)
        let sa = 0;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out_32: u64 = (rd_in & !MASK32) | (rt_in & MASK32);
        let rd_out_64: u64 = sign_extend_u64::<32>(rd_out_32);

        test_rtype_instr(
            "SRA",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SRA",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );
    }

    /// Test the SRAV instruction.
    ///
    /// # SRAV:
    /// ## Type:
    /// - R-Type
    /// ## Operation:
    /// - 32-bit:
    ///   - GPR[rd] <- sign_extend::<32 - sa>(GPR[rt] >> (GPR[rs] & 31))
    /// - 64-bit:
    ///   - temp    <- sign_extend::<32 - sa>(GPR[rt] >> (GPR[rs] & 31))
    ///   - GPR[rd] <- sign_extend_u64::<32>(temp)
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
        let rd_out_32: u64 = (rd_in & !MASK32) | ((rt_in >> (rs_in & 31)) as u32) as u64;
        let rd_out_64: u64 = sign_extend_u64::<32>(rd_out_32);

        test_rtype_instr(
            "SRAV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SRAV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );

        // Shift by 0 (NOP in 32-bit mode, sign extend in 64-bit mode)
        let rs_in: u64 = 0x00000000_00000000;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out_32: u64 = (rd_in & !MASK32) | (rt_in & MASK32);
        let rd_out_64: u64 = sign_extend_u64::<32>(rd_out_32);

        test_rtype_instr(
            "SRAV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SRAV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
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
    ///   - GPR[rd] <- sign_extend::<64 - sa>(GPR[rt] >> sa)
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
            RegSize::Reg32,
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
            RegSize::Reg64,
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
            RegSize::Reg64,
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
            RegSize::Reg32,
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
            RegSize::Reg64,
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
            RegSize::Reg64,
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
            RegSize::Reg32,
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
            RegSize::Reg64,
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
        let rd_out_32: u64 = (rd_in & !MASK32) | ((rt_in as u32) >> sa) as u64;
        let rd_out_64: u64 = sign_extend_u64::<32>(rd_out_32);

        test_rtype_instr(
            "SRL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SRL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );

        // Shift by 0 (sign extend in 64-bit mode)
        let sa = 0;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out_32: u64 = (rd_in & !MASK32) | (rt_in & MASK32);
        let rd_out_64: u64 = sign_extend_u64::<32>(rd_out_32);

        test_rtype_instr(
            "SRL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SRL",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
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
        let rd_out_32: u64 = (rd_in & !MASK32) | ((rt_in as u32) >> (rs_in & 31)) as u64;
        let rd_out_64: u64 = sign_extend_u64::<32>(rd_out_32);

        test_rtype_instr(
            "SRLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SRLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
        );

        // Shift by 0 (sign extend in 64-bit mode)
        let rs_in: u64 = 0x00000000_00000000;
        let rt_in: u64 = 0x00000000_80000000;
        let rd_out_32: u64 = (rd_in & !MASK32) | (rt_in & MASK32);
        let rd_out_64: u64 = sign_extend_u64::<32>(rd_out_32);

        test_rtype_instr(
            "SRLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_32,
            None,
            RegSize::Reg32,
        );

        test_rtype_instr(
            "SRLV",
            OP,
            rs,
            rt,
            rd,
            sa,
            FUNC,
            rs_in,
            rt_in,
            rd_in,
            rd_out_64,
            None,
            RegSize::Reg64,
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
            RegSize::Reg32,
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
            RegSize::Reg64,
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
            RegSize::Reg64,
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
            RegSize::Reg32,
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
            RegSize::Reg64,
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
            RegSize::Reg64,
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
            RegSize::Reg32,
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
            RegSize::Reg64,
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

        let test32 = |rs_in: u64, rt_in: u64| {
            let rs_cmp_32 = rs_in as i32;
            let rt_cmp_32 = rt_in as i32;
            let rd_out_32 = (rd_in & !MASK32) | if rs_cmp_32 < rt_cmp_32 { 1 } else { 0 };

            test_rtype_instr(
                "SLT",
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
        };

        let test64 = |rs_in: u64, rt_in: u64| {
            let rs_cmp_64 = rs_in as i64;
            let rt_cmp_64 = rt_in as i64;
            let rd_out_64 = if rs_cmp_64 < rt_cmp_64 { 1 } else { 0 };

            test_rtype_instr(
                "SLT",
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
        };

        // Condition true
        test32(
            0x00000000_FFFFFFEE, // -18
            0xFFFFFFFF_00000000, // 0
        );
        test64(
            0xFFFFFFFF_FFFFFFEE, // -18
            0x00000000_00000000, // 0
        );

        // Condition false
        test32(
            0xFFFFFFFF_00000000, // 0
            0x00000000_FFFFFFEE, // -18
        );
        test64(
            0x00000000_00000000, // 0
            0xFFFFFFFF_FFFFFFEE, // -18
        );

        // Inputs equal (should produce 0)
        test32(0xABCDEF01_23456789, 0xABCDEF01_23456789);
        test64(0xABCDEF01_23456789, 0xABCDEF01_23456789);
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

        let test32 = |rs_in: u64, imm: u16| {
            let rs_cmp_32: i32 = rs_in as i32;
            let imm_cmp_32: i32 = sign_extend_u32::<16>(imm as u32) as i32;
            let rt_out_32: u64 = (rt_in & !MASK32) | if rs_cmp_32 < imm_cmp_32 { 1 } else { 0 };

            test_itype_instr(
                "SLTI",
                OP,
                rs,
                rt,
                rs_in,
                rt_in,
                imm,
                rt_out_32,
                None,
                RegSize::Reg32,
            );
        };

        let test64 = |rs_in: u64, imm: u16| {
            let rs_cmp_64: i64 = rs_in as i64;
            let imm_cmp_64: i64 = sign_extend_u64::<16>(imm as u64) as i64;
            let rt_out_64: u64 = if rs_cmp_64 < imm_cmp_64 { 1 } else { 0 };

            test_itype_instr(
                "SLTI",
                OP,
                rs,
                rt,
                rs_in,
                rt_in,
                imm,
                rt_out_64,
                None,
                RegSize::Reg64,
            );
        };

        // Condition true
        test32(
            0x00000000_FFFFFFEE, // GPR[rs] = -18
            0x0000,              // imm = 0
        );
        test64(
            0xFFFFFFFF_FFFFFFEE, // GPR[rs] = -18
            0x0000,              // imm = 0
        );

        // Condition false
        test32(
            0x00000000_00000000, // GPR[rs] = 0
            0xFFEE,              // imm = -18
        );
        test64(
            0x00000000_00000000, // GPR[rs] = 0
            0xFFEE,              // imm = -18
        );

        // Inputs equal (should produce 0)
        test32(0x00000000_00007F8A, 0x7F8A);
        test64(0xFFFFFFFF_FFFFFF8A, 0xFF8A);
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

        let test32 = |rs_in: u64, rt_in: u64| {
            let rs_cmp_32: u32 = rs_in as u32;
            let rt_cmp_32: u32 = rt_in as u32;
            let rd_out_32: u64 = (rd_in & !MASK32) | if rs_cmp_32 < rt_cmp_32 { 1 } else { 0 };

            test_rtype_instr(
                "SLTU",
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
        };

        let test64 = |rs_in: u64, rt_in: u64| {
            let rs_cmp_64: u64 = rs_in;
            let rt_cmp_64: u64 = rt_in;
            let rd_out_64: u64 = if rs_cmp_64 < rt_cmp_64 { 1 } else { 0 };

            test_rtype_instr(
                "SLTU",
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
        };

        // Condition true
        test32(
            0xFFFFFFFF_FF000000, // GPR[rs] = 4,278,190,080
            0x00000000_FFFF8000, // GPR[rt] = 4,294,934,528
        );
        test64(0x00000000_FF000000, 0xFFFFFFFF_FFFF8000);

        // Condition false
        test32(0x00000000_00005000, 0x00000000_00004FFF);
        test64(0xFFFFFFFF_FFFFFFFF, 0x00000000_00000000);

        // Inputs equal (should produce 0)
        test32(0x99999999_23232323, 0x10101010_23232323);
        test64(0x34343434_56789ABC, 0x34343434_56789ABC);
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

        let test32 = |rs_in: u64, imm: u16| {
            let rs_cmp_32: u32 = rs_in as u32;
            let imm_cmp_32: u32 = sign_extend_u32::<16>(imm as u32);
            let rt_out_32: u64 = (rt_in & !MASK32) | if rs_cmp_32 < imm_cmp_32 { 1 } else { 0 };

            test_itype_instr(
                "SLTIU",
                OP,
                rs,
                rt,
                rs_in,
                rt_in,
                imm,
                rt_out_32,
                None,
                RegSize::Reg32,
            );
        };

        let test64 = |rs_in: u64, imm: u16| {
            let rs_cmp_64: u64 = rs_in;
            let imm_cmp_64: u64 = sign_extend_u64::<16>(imm as u64);
            let rt_out_64: u64 = if rs_cmp_64 < imm_cmp_64 { 1 } else { 0 };

            test_itype_instr(
                "SLTIU",
                OP,
                rs,
                rt,
                rs_in,
                rt_in,
                imm,
                rt_out_64,
                None,
                RegSize::Reg64,
            );
        };

        // Condition true
        test32(
            0x00000000_FF000000, // GPR[rs] = 4,278,190,080
            0x8000,              // imm = 32,768 -> 4,294,934,528
        );
        test64(0xFFFFFFFF_FF000000, 0x8000);

        // Condition false
        test32(0x00000000_00000000, 0xFFEE);
        test64(
            0x00000000_00000000, // GPR[rs] = 0
            0x0001,              // imm = 1
        );

        // Inputs equal (should produce 0)
        test32(0x00000000_FFFF8000, 0x8000);
        test64(0xFFFFFFFF_FFFF8055, 0x8055);
    }
}
