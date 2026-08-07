//! -----------------------------------------------------------------------
//! disassembler.rs: A basic instruction disassembler for the VR4300
//!
//! Author(s): MrBubblezsz
//! -----------------------------------------------------------------------

use crate::processors::vr4300::{ITypeInstruction, JTypeInstruction, RTypeInstruction};

#[derive(Clone, Copy, Debug)]
enum FormatPart {
    Rd,
    Rs,
    Rt,         // Registers
    OffsetBase, // Decimal offset AND rs register acting as base address.
    // Shown as offset(base)
    Sa,                         // Decimal shift amount
    Target,                     // Target address
    Immediate { signed: bool }, // immediate data
    CoFunc,                     // Coprocessor function
}

impl FormatPart {
    /// Returns the aligned width for this part including the separator
    fn aligned_width(&self, separator_len: usize) -> usize {
        let base_width = match self {
            // Registers: "$r" (2) + up to 2 digits = 4
            FormatPart::Rd | FormatPart::Rt | FormatPart::Rs => 4,
            // Shift amount: up to 2 digits
            FormatPart::Sa => 2,
            // 16-bit hex immediate: "0x" + 4 digits = 6
            FormatPart::Immediate { .. } => 6,
            // These appear last
            FormatPart::Target => 0,
            FormatPart::OffsetBase => 0,
            FormatPart::CoFunc => 0,
        };

        if base_width > 0 {
            base_width + separator_len
        } else {
            0 // No padding
        }
    }
}

enum InstructionFormat {
    IType(&'static str, &'static [FormatPart]),
    JType(&'static str, &'static [FormatPart]),
    RType(&'static str, &'static [FormatPart]),
    Const(&'static str),
}

struct DisassemblerOptions {
    show_target_addresses: bool,
    show_addresses_32_bit: bool,
    no_spaces: bool,
    arg_align: bool,
    mnemonic_pad_wide: bool,
    show_imm_as_hex: bool,
}

#[derive(Default)]
pub struct Disassembler {}

impl Disassembler {
    fn instr_fmt(i: u32) -> InstructionFormat {
        use FormatPart::*;

        let opcode = i >> 26;

        match opcode {
            0 => match i & 0x3F {
                // SPECIAL opcodes
                0 if i == 0 => InstructionFormat::Const("NOP"), // Common instruction used for NOP is SLL $r0,$r0,0
                0 => InstructionFormat::RType("SLL", &[Rd, Rt, Sa]),
                2 => InstructionFormat::RType("SRL", &[Rd, Rt, Sa]),
                3 => InstructionFormat::RType("SRA", &[Rd, Rt, Sa]),
                4 => InstructionFormat::RType("SLLV", &[Rd, Rt, Rd]),
                6 => InstructionFormat::RType("SRLV", &[Rd, Rt, Rs]),
                7 => InstructionFormat::RType("SRAV", &[Rd, Rt, Rs]),
                8 => InstructionFormat::RType("JR", &[Rs]),
                9 => InstructionFormat::RType("JALR", &[Rd, Rs]),
                12 => InstructionFormat::Const("SYSCALL"),
                13 => InstructionFormat::Const("BREAK"),
                15 => InstructionFormat::Const("SYNC"),
                16 => InstructionFormat::RType("MFHI", &[Rd]),
                17 => InstructionFormat::RType("MTHI", &[Rs]),
                18 => InstructionFormat::RType("MFLO", &[Rd]),
                19 => InstructionFormat::RType("MTLO", &[Rs]),
                20 => InstructionFormat::RType("DSLLV", &[Rd, Rt, Rs]),
                22 => InstructionFormat::RType("DSRLV", &[Rd, Rt, Rs]),
                23 => InstructionFormat::RType("DSRAV", &[Rd, Rt, Rs]),
                24 => InstructionFormat::RType("MULT", &[Rs, Rt]),
                25 => InstructionFormat::RType("MULTU", &[Rs, Rt]),
                26 => InstructionFormat::RType("DIV", &[Rs, Rt]),
                27 => InstructionFormat::RType("DIVU", &[Rs, Rt]),
                28 => InstructionFormat::RType("DMULT", &[Rs, Rt]),
                29 => InstructionFormat::RType("DMULTU", &[Rs, Rt]),
                30 => InstructionFormat::RType("DDIV", &[Rs, Rt]),
                31 => InstructionFormat::RType("DDIVU", &[Rs, Rt]),
                32 => InstructionFormat::RType("ADD", &[Rd, Rs, Rt]),
                33 => InstructionFormat::RType("ADDU", &[Rd, Rs, Rt]),
                34 => InstructionFormat::RType("SUB", &[Rd, Rs, Rt]),
                35 => InstructionFormat::RType("SUBU", &[Rd, Rs, Rt]),
                36 => InstructionFormat::RType("AND", &[Rd, Rs, Rt]),
                37 => InstructionFormat::RType("OR", &[Rd, Rs, Rt]),
                38 => InstructionFormat::RType("XOR", &[Rd, Rs, Rt]),
                39 => InstructionFormat::RType("NOR", &[Rd, Rs, Rt]),
                42 => InstructionFormat::RType("SLT", &[Rd, Rs, Rt]),
                43 => InstructionFormat::RType("SLTU", &[Rd, Rs, Rt]),
                44 => InstructionFormat::RType("DADD", &[Rd, Rs, Rt]),
                45 => InstructionFormat::RType("DADDU", &[Rd, Rs, Rt]),
                46 => InstructionFormat::RType("DSUB", &[Rd, Rs, Rt]),
                47 => InstructionFormat::RType("DSUBU", &[Rd, Rs, Rt]),
                48 => InstructionFormat::RType("TGE", &[Rs, Rt]),
                49 => InstructionFormat::RType("TGEU", &[Rs, Rt]),
                50 => InstructionFormat::RType("TLT", &[Rs, Rt]),
                51 => InstructionFormat::RType("TLTU", &[Rs, Rt]),
                52 => InstructionFormat::RType("TEQ", &[Rs, Rt]),
                54 => InstructionFormat::RType("TNE", &[Rs, Rt]),
                56 => InstructionFormat::RType("DSLL", &[Rd, Rt, Rs]),
                58 => InstructionFormat::RType("DSRL", &[Rd, Rt, Rs]),
                59 => InstructionFormat::RType("DSRA", &[Rd, Rt, Rs]),
                60 => InstructionFormat::RType("DSLL32", &[Rd, Rt, Rs]),
                62 => InstructionFormat::RType("DSRL32", &[Rd, Rt, Rs]),
                63 => InstructionFormat::RType("DSRA32", &[Rd, Rt, Rs]),
                _ => InstructionFormat::Const("Reserved Instruction"),
            },
            1 => {
                // REGIMM
                let rt = (i >> 16) & 0x1F;

                match rt {
                    0 => InstructionFormat::IType("BLTZ", &[Rs, Target]),
                    1 => InstructionFormat::IType("BGEZ", &[Rs, Target]),
                    2 => InstructionFormat::IType("BLTZL", &[Rs, Target]),
                    3 => InstructionFormat::IType("BGEZL", &[Rs, Target]),
                    8 => InstructionFormat::IType("TGEI", &[Rs, Immediate { signed: true }]),
                    9 => InstructionFormat::IType("TGEIU", &[Rs, Immediate { signed: false }]),
                    10 => InstructionFormat::IType("TLTI", &[Rs, Immediate { signed: true }]),
                    11 => InstructionFormat::IType("TLTIU", &[Rs, Immediate { signed: false }]),
                    12 => InstructionFormat::IType("TEQI", &[Rs, Immediate { signed: true }]),
                    14 => InstructionFormat::IType("TNEI", &[Rs, Immediate { signed: true }]),
                    16 => InstructionFormat::IType("BLTZAL", &[Rs, Target]),
                    17 => InstructionFormat::IType("BGEZAL", &[Rs, Target]),
                    18 => InstructionFormat::IType("BLTZALL", &[Rs, Target]),
                    19 => InstructionFormat::IType("BGEZALL", &[Rs, Target]),
                    _ => InstructionFormat::Const("Reserved Instruction"),
                }
            }
            2 => InstructionFormat::JType("J", &[Target]),
            3 => InstructionFormat::JType("JAL", &[Target]),
            4 => InstructionFormat::IType("BEQ", &[Rs, Rt, Target]),
            5 => InstructionFormat::IType("BNE", &[Rs, Rt, Target]),
            6 => InstructionFormat::IType("BLEZ", &[Rs, Target]),
            7 => InstructionFormat::IType("BGTZ", &[Rs, Target]),
            8 => InstructionFormat::IType("ADDI", &[Rt, Rs, Immediate { signed: true }]),
            9 => InstructionFormat::IType("ADDIU", &[Rt, Rs, Immediate { signed: false }]),
            10 => InstructionFormat::IType("SLTI", &[Rt, Rs, Immediate { signed: true }]),
            11 => InstructionFormat::IType("SLTIU", &[Rt, Rs, Immediate { signed: false }]),
            12 => InstructionFormat::IType("ANDI", &[Rt, Rs, Immediate { signed: true }]),
            13 => InstructionFormat::IType("ORI", &[Rt, Rs, Immediate { signed: true }]),
            14 => InstructionFormat::IType("XORI", &[Rt, Rs, Immediate { signed: true }]),
            15 => InstructionFormat::IType("LUI", &[Rt, Immediate { signed: true }]),
            16 => InstructionFormat::JType("COP0", &[CoFunc]),
            17 => InstructionFormat::JType("COP1", &[CoFunc]),
            18 => InstructionFormat::JType("COP2", &[CoFunc]),
            20 => InstructionFormat::IType("BEQL", &[Rs, Rt, Target]),
            21 => InstructionFormat::IType("BNEL", &[Rs, Rt, Target]),
            22 => InstructionFormat::IType("BLEZL", &[Rs, Rt, Target]),
            23 => InstructionFormat::IType("BGTZL", &[Rs, Rt, Target]),
            24 => InstructionFormat::IType("DADDI", &[Rt, Rs, Immediate { signed: true }]),
            25 => InstructionFormat::IType("DADDIU", &[Rt, Rs, Immediate { signed: false }]),
            26 => InstructionFormat::IType("LDL", &[Rt, OffsetBase]),
            27 => InstructionFormat::IType("LDR", &[Rt, OffsetBase]),
            32 => InstructionFormat::IType("LB", &[Rt, OffsetBase]),
            33 => InstructionFormat::IType("LH", &[Rt, OffsetBase]),
            34 => InstructionFormat::IType("LWL", &[Rt, OffsetBase]),
            35 => InstructionFormat::IType("LW", &[Rt, OffsetBase]),
            36 => InstructionFormat::IType("LBU", &[Rt, OffsetBase]),
            37 => InstructionFormat::IType("LHU", &[Rt, OffsetBase]),
            38 => InstructionFormat::IType("LWR", &[Rt, OffsetBase]),
            39 => InstructionFormat::IType("LWU", &[Rt, OffsetBase]),
            40 => InstructionFormat::IType("SB", &[Rt, OffsetBase]),
            41 => InstructionFormat::IType("SH", &[Rt, OffsetBase]),
            42 => InstructionFormat::IType("SWL", &[Rt, OffsetBase]),
            43 => InstructionFormat::IType("SW", &[Rt, OffsetBase]),
            44 => InstructionFormat::IType("SDL", &[Rt, OffsetBase]),
            45 => InstructionFormat::IType("SDR", &[Rt, OffsetBase]),
            46 => InstructionFormat::IType("SWR", &[Rt, OffsetBase]),
            47 => InstructionFormat::Const("CACHE"),
            48 => InstructionFormat::IType("LL", &[Rt, OffsetBase]),
            49 => InstructionFormat::IType("LWC1", &[Rt, OffsetBase]),
            50 => InstructionFormat::IType("LWC2", &[Rt, OffsetBase]),
            52 => InstructionFormat::IType("LLD", &[Rt, OffsetBase]),
            53 => InstructionFormat::IType("LDC1", &[Rt, OffsetBase]),
            54 => InstructionFormat::IType("LDC1", &[Rt, OffsetBase]),
            55 => InstructionFormat::IType("LD", &[Rt, OffsetBase]),
            56 => InstructionFormat::IType("SC", &[Rt, OffsetBase]),
            57 => InstructionFormat::IType("SWC1", &[Rt, OffsetBase]),
            58 => InstructionFormat::IType("SWC2", &[Rt, OffsetBase]),
            60 => InstructionFormat::IType("SCD", &[Rt, OffsetBase]),
            61 => InstructionFormat::IType("SDC1", &[Rt, OffsetBase]),
            62 => InstructionFormat::IType("SDC1", &[Rt, OffsetBase]),
            63 => InstructionFormat::IType("SD", &[Rt, OffsetBase]),
            _ => InstructionFormat::Const("Reserved Instruction"),
        }
    }

    pub fn instruction_string(pc: u64, i: u32) -> String {
        let options = DisassemblerOptions {
            show_target_addresses: true,
            show_addresses_32_bit: true,
            no_spaces: true,
            arg_align: true,
            mnemonic_pad_wide: false,
            show_imm_as_hex: false,
        };

        const MAX_MNEMONIC_LEN: usize = 7; // BLTZALL
        const COMMON_MNEMONIC_LEN: usize = 5;

        let mnemonic_pad_len = if options.mnemonic_pad_wide {
            MAX_MNEMONIC_LEN
        } else {
            COMMON_MNEMONIC_LEN
        };

        let format_mnemonic = |mnemonic: &str| {
            let m_pad = mnemonic_pad_len.saturating_sub(mnemonic.len());
            format!("{}{}", mnemonic, " ".repeat(1 + m_pad))
        };

        match Disassembler::instr_fmt(i) {
            InstructionFormat::Const(mnumonic) => mnumonic.to_string(),
            InstructionFormat::IType(mnemonic, ifmt) => {
                let instr = ITypeInstruction::from_raw(i);
                format_mnemonic(mnemonic)
                    + &format_args(ifmt, &options, |part| fmt_ipart(pc, &options, &instr, part))
            }

            InstructionFormat::RType(mnemonic, rfmt) => {
                let instr = RTypeInstruction::from_raw(i);
                format_mnemonic(mnemonic)
                    + &format_args(rfmt, &options, |part| fmt_rpart(pc, &options, &instr, part))
            }

            InstructionFormat::JType(mnemonic, jfmt) => {
                let instr = JTypeInstruction::from_raw(i);
                format_mnemonic(mnemonic)
                    + &format_args(jfmt, &options, |part| fmt_jpart(pc, &options, &instr, part))
            }
        }
    }
}

fn format_args<F>(parts: &[FormatPart], options: &DisassemblerOptions, mut fmt_fn: F) -> String
where
    F: FnMut(FormatPart) -> String,
{
    let separator = if options.no_spaces { "," } else { ", " };
    let mut result = String::new();

    for (i, &part) in parts.iter().enumerate() {
        let is_last = i + 1 == parts.len();
        let formatted = fmt_fn(part);

        if is_last {
            result += &formatted;
        } else {
            let with_sep = format!("{}{}", formatted, separator);
            if options.arg_align {
                let width = part.aligned_width(separator.len());
                result += &format!("{:<width$}", with_sep);
            } else {
                result += &with_sep;
            }
        }
    }

    result
}

fn fmt_ipart(
    pc: u64,
    options: &DisassemblerOptions,
    instr: &ITypeInstruction,
    part: FormatPart,
) -> String {
    match part {
        FormatPart::Rt => format!("$r{}", instr.rt),
        FormatPart::Rs => format!("$r{}", instr.rs),
        FormatPart::Immediate { signed } => {
            if options.show_imm_as_hex {
                format!("0x{:04X}", instr.imm)
            } else {
                if signed {
                    format!("{}", instr.imm as i16)
                } else {
                    format!("{}", instr.imm)
                }
            }
        }
        FormatPart::OffsetBase => format!("{}(${})", instr.imm as i16, instr.rs),
        FormatPart::Target => {
            if options.show_target_addresses {
                if options.show_addresses_32_bit {
                    let tgt = (pc as u32)
                        .wrapping_add(((instr.imm as i16) as u32) << 2)
                        .wrapping_add(4);
                    format!("tgt_{:08X}", tgt)
                } else {
                    let tgt = pc
                        .wrapping_add(((instr.imm as i16) as u64) << 2)
                        .wrapping_add(4);
                    format!("tgt_{:016X}", tgt)
                }
            } else {
                format!("0x{:04X}", instr.imm)
            }
        }
        _ => unreachable!("No I-Type instruction uses other types of data"),
    }
}

fn fmt_rpart(
    _pc: u64,
    _options: &DisassemblerOptions,
    instr: &RTypeInstruction,
    part: FormatPart,
) -> String {
    match part {
        FormatPart::Rd => format!("$r{}", instr.rd),
        FormatPart::Rt => format!("$r{}", instr.rt),
        FormatPart::Rs => format!("$r{}", instr.rs),
        FormatPart::Sa => format!("{}", instr.shift),
        _ => unreachable!("No R-Type instruction uses other types of data"),
    }
}

fn fmt_jpart(
    pc: u64,
    options: &DisassemblerOptions,
    instr: &JTypeInstruction,
    part: FormatPart,
) -> String {
    match part {
        FormatPart::Target => {
            if options.show_target_addresses {
                if options.show_addresses_32_bit {
                    let tgt = (pc & 0xF0000000) as u32 | (instr.target << 2);
                    format!("tgt_{:08X}", tgt)
                } else {
                    let tgt = (pc & 0xFFFFFFFF_F0000000) | (instr.target << 2) as u64;
                    format!("tgt_{:016X}", tgt)
                }
            } else {
                format!("0x{:07X}", instr.target)
            }
        }
        FormatPart::CoFunc => format!("0x{:07X}", instr.target & 0x1FFFFFF),
        _ => unreachable!("No J-Type instruction uses other types of data"),
    }
}
