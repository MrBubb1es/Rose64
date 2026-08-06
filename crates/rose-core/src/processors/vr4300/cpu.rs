//! -----------------------------------------------------------------------
//! vr4300/cpu.rs: Main CPU interpreter implementation
//!
//! A basic implementation of the N64's main CPU. This is a cycle-accurate
//! interpreter implementation: slow, but correct.
//!
//! Author(s): MrBubblezsz, logocrazymon
//! -----------------------------------------------------------------------

use crate::{
    common::hint::rose_unlikely,
    memory::bus::{Bus, MemoryAccess},
};

/// Representation of the N64's VR4300 processor.
#[repr(C)] // Stable layout needed so JIT code can index fields by offset
#[derive(Default)]
pub struct CpuVR4300 {
    pub gpr: [u64; 32],
    pub fpr: [f64; 32],
    pub pc: u64,
    pub mult_hi: u64,
    pub mult_lo: u64,
    pub fp_revision: f32,
    pub fp_control: f32,
    pub llbit: bool,
    pub reg_size: RegSize,
    pub last_exception: Option<CpuException>,
}

/// Data for an I-type instruction. An I-Type instruction has the structure:
///
/// <pre>
/// +-----------------+-------+-------+----------------+
/// | opcode (6 bits) | rs(5) | rt(5) | immediate (16) |
/// +-----------------+-------+-------+----------------+
/// </pre>
struct ITypeInstruction {
    opcode: u8,
    rs: u8,
    rt: u8,
    immediate: u16,
}

impl ITypeInstruction {
    /// Take in a raw 32-bit I-type instruction and return a parsed
    /// ITypeInstruction struct.
    ///
    /// # Argument:
    /// - The instruction (u32)
    /// # Returns:
    /// - `ITypeInstruction` struct with separated fields
    #[inline]
    fn from_raw(i: u32) -> ITypeInstruction {
        ITypeInstruction {
            opcode: (i >> 26) as u8,
            rs: ((i >> 21) & 0x1F) as u8,
            rt: ((i >> 16) & 0x1F) as u8,
            immediate: i as u16,
        }
    }
}

/// Data for a J-type opcode. A J-Type instruction has the structure:
/// <pre>
/// +-----------------+------------------------------------+
/// | opcode (6 bits) |          target (26 bits)          |
/// +-----------------+------------------------------------+
/// </pre>
struct JTypeInstruction {
    opcode: u8,
    target: u32,
}

impl JTypeInstruction {
    /// Take in a raw 32-bit J-type instruction and return a parsed
    /// JTypeInstruction struct.
    ///
    /// # Argument:
    /// - The instruction (u32)
    /// # Returns:
    /// - `JTypeInstruction` struct with separated fields
    #[inline]
    fn from_raw(i: u32) -> JTypeInstruction {
        JTypeInstruction {
            opcode: (i >> 26) as u8,
            target: i & 0x3FFFFFF,
        }
    }
}

/// Data for an R-type opcode. An R-Type instruction has the structure:
/// <pre>
/// +-----------------+-------+-------+-------+-------+----------+
/// | opcode (6 bits) | rs(5) | rt(5) | rd(5) | sa(5) | funct(6) |
/// +-----------------+-------+-------+-------+-------+----------+
/// </pre>
struct RTypeInstruction {
    opcode: u8,
    rs: u8,
    rt: u8,
    rd: u8,
    shift: u8,
    funct: u8,
}

impl RTypeInstruction {
    /// Take in a raw 32-bit R-type instruction and return a parsed
    /// RTypeInstruction struct.
    ///
    /// # Argument:
    /// - The instruction (u32)
    /// # Returns:
    /// - `RTypeInstruction` struct with separated fields
    #[inline]
    fn from_raw(i: u32) -> RTypeInstruction {
        RTypeInstruction {
            opcode: (i >> 26) as u8,
            rs: ((i >> 21) & 0x1F) as u8,
            rt: ((i >> 16) & 0x1F) as u8,
            rd: ((i >> 11) & 0x1F) as u8,
            shift: ((i >> 6) & 0x1F) as u8,
            funct: (i & 0x3F) as u8,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum RegSize {
    #[default]
    Reg32,
    Reg64,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CpuException {
    IntegerOverflow,
    ReservedInstruction,
    AddressErrorLoad,
    AddressErrorStore,
    
}

impl CpuVR4300 {
    /// Index of the Zero Register in the regs array
    pub const ZR: usize = 0;
    /// Index of the Link Register in the regs array
    pub const LR: usize = 31;

    pub fn new() -> CpuVR4300 {
        CpuVR4300::default()
    }

    pub fn execute_instruction(&mut self, bus: &mut Bus, i: u32) -> Option<CpuException> {
        let opcode = i >> 26;

        match opcode {
            0 => match i & 0x3F {
                // SPECIAL opcodes
                0 => {
                    // SLL
                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];

                    self.gpr[instr.rd as usize] = (((rt as u32) << instr.shift) as i32) as u64;
                }
                2 => {
                    // SRL
                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];

                    self.gpr[instr.rd as usize] = (((rt as u32) >> instr.shift) as i32) as u64;
                }
                3 => {
                    // SRA
                    //
                    // SRA HARDWARE BUG (32-bit mode):
                    //   Instead of shifting in 1's or 0's based on the sign of the 32-bit
                    //   GPR[rt] value, bits from the high 32-bits are shifted in instead.
                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];

                    self.gpr[instr.rd as usize] = (((rt as i64) >> instr.shift) as i32) as u64;
                }
                4 => {
                    // SLLV
                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let shift = self.gpr[instr.rs as usize] & 0x1F;

                    self.gpr[instr.rd as usize] = (((rt as u32) << shift) as i32) as u64;
                }
                6 => {
                    // SRLV
                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let shift = self.gpr[instr.rs as usize] & 0x1F;

                    self.gpr[instr.rd as usize] = (((rt as u32) >> shift) as i32) as u64;
                }
                7 => {
                    // SRAV
                    //
                    // SRAV HARDWARE BUG (32-bit mode):
                    //   Instead of shifting in 1's or 0's based on the sign of the 32-bit
                    //   GPR[rt] value, bits from the high 32-bits are shifted in instead.
                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let shift = self.gpr[instr.rs as usize] & 0x1F;

                    self.gpr[instr.rd as usize] = (((rt as i64) >> shift) as i32) as u64;
                }
                8 => {}  // JR
                9 => {}  // JALR
                12 => {} // SYSCALL
                13 => {} // BRK
                15 => {} // SYNC
                16 => {} // MFHI
                17 => {} // MTHI
                18 => {} // MFLO
                19 => {} // MTLO
                20 => {
                    // DSLLV
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let shift = self.gpr[instr.rs as usize] & 0x3F;
                    let result = rt << shift;
                    self.gpr[instr.rd as usize] = result;
                }
                22 => {
                    // DSRLV
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let shift = self.gpr[instr.rs as usize] & 0x3F;
                    let result = rt >> shift;
                    self.gpr[instr.rd as usize] = result;
                }
                23 => {
                    // DSRAV
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    // Cast to signed for arithmetic shift
                    let rt = self.gpr[instr.rt as usize] as i64;
                    let shift = self.gpr[instr.rs as usize] & 0x3F;
                    let result = rt >> shift;
                    self.gpr[instr.rd as usize] = result as u64;
                }
                24 => {
                    // MULT
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as i32;
                    let rt = self.gpr[instr.rt as usize] as i32;
                    let prod = rs as i64 * rt as i64;
                    self.mult_lo = (prod as i32) as u64;
                    self.mult_hi = ((prod >> 32) as i32) as u64;
                }
                25 => {
                    // MULTU
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as u32;
                    let rt = self.gpr[instr.rt as usize] as u32;
                    let prod = rs as i64 * rt as i64;
                    self.mult_lo = (prod as i32) as u64;
                    self.mult_hi = ((prod >> 32) as i32) as u64;
                }
                26 => {
                    // DIV
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as i32;
                    let rt = self.gpr[instr.rt as usize] as i32;
                    let q = rs.checked_div(rt).unwrap_or(if rs < 0 { 1 } else { -1 });
                    let r = rs.checked_rem(rt).unwrap_or(rs);

                    self.mult_lo = q as u64;
                    self.mult_hi = r as u64;
                }
                27 => {
                    // DIVU
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as u32;
                    let rt = self.gpr[instr.rt as usize] as u32;
                    let q = rs.checked_div(rt).unwrap_or(u32::MAX);
                    let r = rs.checked_rem(rt).unwrap_or(rs);

                    self.mult_lo = (q as i32) as u64;
                    self.mult_hi = (r as i32) as u64;
                }
                28 => {
                    // DMULT
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rs = (self.gpr[instr.rs as usize] as i64) as i128;
                    let rt = (self.gpr[instr.rt as usize] as i64) as i128;
                    let prod = rs * rt;
                    self.mult_lo = prod as u64;
                    self.mult_hi = (prod >> 64) as u64;
                }
                29 => {
                    // DMULTU
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as u128;
                    let rt = self.gpr[instr.rt as usize] as u128;
                    let prod = rs * rt;
                    self.mult_lo = prod as u64;
                    self.mult_hi = (prod >> 64) as u64;
                }
                30 => {
                    // DDIV
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;
                    let q = rs.checked_div(rt).unwrap_or(if rs < 0 { 1 } else { -1 });
                    let r = rs.checked_rem(rt).unwrap_or(rs);

                    self.mult_lo = q as u64;
                    self.mult_hi = r as u64;
                }
                31 => {
                    // DDIVU
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize];
                    let rt = self.gpr[instr.rt as usize];
                    let q = rs.checked_div(rt).unwrap_or(u64::MAX);
                    let r = rs.checked_rem(rt).unwrap_or(rs);

                    self.mult_lo = q;
                    self.mult_hi = r;
                }
                32 => {
                    // ADD
                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize] as i32;
                    let rt = self.gpr[instr.rt as usize] as i32;
                    let (sum, overflow) = rs.overflowing_add(rt);

                    if overflow {
                        return Some(CpuException::IntegerOverflow);
                    }

                    self.gpr[instr.rd as usize] = sum as u64;
                }
                33 => {
                    // ADDU
                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize] as i32;
                    let rt = self.gpr[instr.rt as usize] as i32;
                    let sum = rs.wrapping_add(rt);

                    self.gpr[instr.rd as usize] = sum as u64;
                }
                34 => {
                    // SUB
                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize] as i32;
                    let rt = self.gpr[instr.rt as usize] as i32;
                    let (diff, overflow) = rs.overflowing_sub(rt);

                    if overflow {
                        return Some(CpuException::IntegerOverflow);
                    }
                    
                    self.gpr[instr.rd as usize] = diff as u64;
                }
                35 => {
                    // SUBU
                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize] as i32;
                    let rt = self.gpr[instr.rt as usize] as i32;
                    let diff = rs.wrapping_sub(rt);

                    self.gpr[instr.rd as usize] = diff as u64;
                }
                36 => {
                    // AND
                    let instr = RTypeInstruction::from_raw(i);

                    self.gpr[instr.rd as usize] =
                        self.gpr[instr.rt as usize] & self.gpr[instr.rs as usize];
                }
                37 => {
                    // OR
                    let instr = RTypeInstruction::from_raw(i);

                    self.gpr[instr.rd as usize] =
                        self.gpr[instr.rt as usize] | self.gpr[instr.rs as usize];
                }
                38 => {
                    // XOR
                    let instr = RTypeInstruction::from_raw(i);

                    self.gpr[instr.rd as usize] =
                        self.gpr[instr.rt as usize] ^ self.gpr[instr.rs as usize];
                }
                39 => {
                    // NOR
                    let instr = RTypeInstruction::from_raw(i);

                    self.gpr[instr.rd as usize] =
                        !(self.gpr[instr.rt as usize] | self.gpr[instr.rs as usize]);
                }
                42 => {
                    // SLT
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;
                    self.gpr[instr.rd as usize] = if rs < rt { 1u64 } else { 0u64 };
                }
                43 => {
                    // SLTU
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize];
                    let rt = self.gpr[instr.rt as usize];
                    self.gpr[instr.rd as usize] = if rs < rt { 1u64 } else { 0u64 };
                }
                44 => {
                    // DADD
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize] as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;

                    let (result, overflow) = rs.overflowing_add(rt);

                    if overflow {
                        return Some(CpuException::IntegerOverflow);
                    }

                    self.gpr[instr.rd as usize] = result as u64;
                }
                45 => {
                    // DADDU
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize];
                    let rt = self.gpr[instr.rt as usize];

                    let result = rs.wrapping_add(rt);

                    self.gpr[instr.rd as usize] = result;
                }
                46 => {
                    // DSUB
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize] as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;

                    let (result, overflow) = rs.overflowing_sub(rt);

                    if overflow {
                        return Some(CpuException::IntegerOverflow);
                    }

                    self.gpr[instr.rd as usize] = result as u64;
                }
                47 => {
                    // DSUBU
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize];
                    let rt = self.gpr[instr.rt as usize];

                    let result = rs.wrapping_sub(rt);

                    self.gpr[instr.rd as usize] = result;
                }
                48 => {} // TGE
                49 => {} // TGEU
                50 => {} // TLT
                51 => {} // TLTU
                52 => {} // TEQ
                54 => {} // TNE
                56 => {
                    // DSLL
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let result = rt << instr.shift;
                    self.gpr[instr.rd as usize] = result;
                }
                58 => {
                    // DSRL
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let result = rt >> instr.shift;
                    self.gpr[instr.rd as usize] = result;
                }
                59 => {
                    // DSRA
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    // Cast to signed value for arithmetic shift.
                    let rt = self.gpr[instr.rt as usize] as i64;
                    let result = rt >> instr.shift;
                    self.gpr[instr.rd as usize] = result as u64;
                }
                60 => {
                    // DSLL32
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let result = rt << (instr.shift + 32);
                    self.gpr[instr.rd as usize] = result;
                }
                62 => {
                    // DSRL32
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let result = rt >> (instr.shift + 32);
                    self.gpr[instr.rd as usize] = result;
                }
                63 => {
                    // DSRA32
                    if self.reg_size == RegSize::Reg32 {
                        return Some(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    // Cast to signed for arithmetic shift
                    let rt = self.gpr[instr.rt as usize] as i64;
                    let result = rt >> (instr.shift + 32);
                    self.gpr[instr.rd as usize] = result as u64;
                }
                _ => panic!("Unrecognized SPECIAL opcode {i}"),
            },
            1 => {} // REGIMM
            2 => {} // J
            3 => {} // JAL
            4 => {} // BEQ
            5 => {} // BNE
            6 => {} // BLEZ
            7 => {} // BGTZ
            8 => {
                // ADDI
                let instr = ITypeInstruction::from_raw(i);

                let rs = self.gpr[instr.rs as usize] as i32;
                let immediate = (instr.immediate as i16) as i32;
                let (sum, overflow) = rs.overflowing_add(immediate);

                if overflow {
                    return Some(CpuException::IntegerOverflow);
                }

                self.gpr[instr.rt as usize] = sum as u64;
            }
            9 => {
                // ADDIU
                let instr = ITypeInstruction::from_raw(i);

                let rs = self.gpr[instr.rs as usize] as i32;
                let immediate = (instr.immediate as i16) as i32;
                let sum = rs.wrapping_add(immediate);

                self.gpr[instr.rt as usize] = sum as u64;
            }
            10 => {
                // SLTI
                let instr = ITypeInstruction::from_raw(i);
                let rs = self.gpr[instr.rs as usize] as i64;
                let imm = (instr.immediate as i16) as i64;
                self.gpr[instr.rt as usize] = if rs < imm { 1u64 } else { 0u64 };
            }
            11 => {
                // SLTIU
                let instr = ITypeInstruction::from_raw(i);
                let rs = self.gpr[instr.rs as usize];
                let imm = instr.immediate as u64;
                self.gpr[instr.rt as usize] = if rs < imm { 1u64 } else { 0u64 };
            }
            12 => {
                // ANDI
                let instr = ITypeInstruction::from_raw(i);

                self.gpr[instr.rt as usize] = instr.immediate as u64 & self.gpr[instr.rs as usize];
            }
            13 => {
                // ORI
                let instr = ITypeInstruction::from_raw(i);

                self.gpr[instr.rt as usize] = instr.immediate as u64 | self.gpr[instr.rs as usize];
            }
            14 => {
                // XORI
                let instr = ITypeInstruction::from_raw(i);

                self.gpr[instr.rt as usize] = instr.immediate as u64 ^ self.gpr[instr.rs as usize];
            }
            15 => {
                // LUI
                let instr = ITypeInstruction::from_raw(i);
                let val = (((instr.immediate as u32) << 16) as i32) as u64;
                self.gpr[instr.rt as usize] = val;
            }
            16 => {} // COP0
            17 => {} // COP1
            18 => {} // COP2
            20 => {} // BEQL
            21 => {} // BNEL
            22 => {} // BLEZL
            23 => {} // BGTZL
            24 => {
                // DADDI
                if self.reg_size == RegSize::Reg32 {
                    return Some(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let rs = self.gpr[instr.rs as usize] as i64;
                let immediate = (instr.immediate as i16) as i64;
                let (result, overflow) = rs.overflowing_add(immediate);
                
                if overflow {
                    return Some(CpuException::IntegerOverflow);
                }

                self.gpr[instr.rt as usize] = result as u64;
            }
            25 => {
                // DADDIU
                if self.reg_size == RegSize::Reg32 {
                    return Some(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let rs = self.gpr[instr.rs as usize];
                let immediate = (instr.immediate as i16) as u64;
                let result = rs.wrapping_add(immediate);
                self.gpr[instr.rt as usize] = result;
            }
            26 => {
                // LDL
                if self.reg_size == RegSize::Reg32 {
                    return Some(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.immediate as i16) as u64;
                let addr = self.gpr[instr.rs as usize] + offset;
                let addr_aligned = addr & !7;
                let byte_offset = addr & 7;
                let value = match self.read64(bus, addr_aligned) {
                    Ok(data) => data,
                    Err(e) => return Some(e),
                };

                // let result = match byte_offset {
                //     0 => 0x00000000_00000000 | (value >> 0),
                //     1 => 0xFF000000_00000000 | (value >> 8),
                //     2 => 0xFFFF0000_00000000 | (value >> 16),
                //     3 => 0xFFFFFF00_00000000 | (value >> 24),
                //     4 => 0xFFFFFFFF_00000000 | (value >> 32),
                //     5 => 0xFFFFFFFF_FF000000 | (value >> 40),
                //     6 => 0xFFFFFFFF_FFFF0000 | (value >> 48),
                //     7 => 0xFFFFFFFF_FFFFFF00 | (value >> 56),
                //     _ => unreachable!()
                // };

                // Same result as match but no branching
                let temp = 8 * (byte_offset + 1);
                let mask = 0xFFFFFFFF_FFFFFFFFu64.checked_shr(temp as u32).unwrap_or(0);
                let shift = 64 - temp;
                self.gpr[instr.rt as usize] = (self.gpr[instr.rt as usize] & mask) | (value << shift);
            }
            27 => {
                // LDR
                if self.reg_size == RegSize::Reg32 {
                    return Some(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.immediate as i16) as u64;
                let addr = self.gpr[instr.rs as usize] + offset;
                let addr_aligned = addr & !7;
                let byte_offset = addr & 7;
                let value = match self.read64(bus, addr_aligned) {
                    Ok(data) => data,
                    Err(e) => return Some(e),
                };

                // let result = match byte_offset {
                //     0 => 0x00FFFFFF_FFFFFFFF | (value << 56),
                //     1 => 0x0000FFFF_FFFFFFFF | (value << 48),
                //     2 => 0x000000FF_FFFFFFFF | (value << 40),
                //     3 => 0x00000000_FFFFFFFF | (value << 32),
                //     4 => 0x00000000_00FFFFFF | (value << 24),
                //     5 => 0x00000000_0000FFFF | (value << 16),
                //     6 => 0x00000000_000000FF | (value << 8),
                //     7 => 0x00000000_00000000 | (value << 0),
                //     _ => unreachable!()
                // };

                // Same result as match but no branching
                let temp = 8 * byte_offset;
                let mask = 0xFFFFFFFF_FFFFFFFFu64.checked_shr(64 - temp as u32).unwrap_or(0);
                let shift = temp;
                self.gpr[instr.rt as usize] = (self.gpr[instr.rt as usize] & mask) | (value >> shift);
            }
            32 => {
                // LB
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.immediate as i16) as u64;
                let addr = self.gpr[instr.rs as usize] + offset;

                match self.read8(bus, addr) {
                    Ok(data) => self.gpr[instr.rt as usize] = (data as i8) as u64,
                    Err(e) => return Some(e),
                }
            }
            33 => {
                // LH
                let instr = ITypeInstruction::from_raw(i);
                let base = self.gpr[instr.rs as usize] as i64;
                let offset = (instr.immediate as i16) as i64;
                let addr = (base + offset) as u64;
                match self.read16(bus, addr) {
                    Ok(data) => self.gpr[instr.rt as usize] = (data as i16) as u64,
                    Err(e) => return Some(e),
                }
            }
            34 => {
                // LWL
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.immediate as i16) as u64;
                let addr = self.gpr[instr.rs as usize] + offset;
                let addr_aligned = addr & !3;
                let byte_offset = addr & 3;
                let value = match self.read32(bus, addr_aligned) {
                    Ok(data) => data,
                    Err(e) => return Some(e), // TODO: probably cold path
                };

                // let result = match byte_offset {
                //     0 => 0x00FFFFFF | (value << 24),
                //     1 => 0x0000FFFF | (value << 16),
                //     2 => 0x000000FF | (value << 8),
                //     3 => 0x00000000 | (value << 0),
                //     _ => unreachable!()
                // };

                // Same result as match but no branching
                let temp = 4 * (byte_offset + 1);
                let mask = 0xFFFFFFFFu32.checked_shr(temp as u32).unwrap_or(0);
                let shift = 32 - temp;
                let reg_in = self.gpr[instr.rs as usize] as u32;
                let result = (reg_in & mask) | (value << shift);
                self.gpr[instr.rt as usize] = (result as i32) as u64;
            }
            35 => {
                // LW
                let instr = ITypeInstruction::from_raw(i);
                let base = self.gpr[instr.rs as usize] as i64;
                let offset = (instr.immediate as i16) as i64;
                let addr = (base + offset) as u64;
                match self.read32(bus, addr) {
                    Ok(data) => self.gpr[instr.rt as usize] = (data as i32) as u64,
                    Err(e) => return Some(e),
                }
            }
            36 => {
                // LBU
                let instr = ITypeInstruction::from_raw(i);
                let base = self.gpr[instr.rs as usize] as i64;
                let offset = (instr.immediate as i16) as i64;
                let addr = (base + offset) as u64;
                match self.read8(bus, addr) {
                    Ok(data) => self.gpr[instr.rt as usize] = data as u64,
                    Err(e) => return Some(e),
                }
            }
            37 => {
                // LHU
                let instr = ITypeInstruction::from_raw(i);
                let base = self.gpr[instr.rs as usize] as i64;
                let offset = (instr.immediate as i16) as i64;
                let addr = (base + offset) as u64;
                match self.read16(bus, addr) {
                    Ok(data) => self.gpr[instr.rt as usize] = data as u64,
                    Err(e) => return Some(e),
                }
            }
            38 => {
                // LWR
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.immediate as i16) as u64;
                let addr = self.gpr[instr.rs as usize] + offset;
                let addr_aligned = addr & !3;
                let byte_offset = addr & 3;
                let value = match self.read32(bus, addr_aligned) {
                    Ok(data) => data,
                    Err(e) => return Some(e),
                };

                // let result = match byte_offset {
                //     0 => 0x00000000 | (value >> 0),
                //     1 => 0xFF000000 | (value >> 8),
                //     2 => 0xFFFF0000 | (value >> 16),
                //     3 => 0xFFFFFF00 | (value >> 24),
                //     _ => unreachable!()
                // };

                // Same result as match but no branching
                let temp = 4 * byte_offset;
                let mask = 0xFFFFFFFFu32.checked_shl(temp as u32).unwrap_or(0);
                let shift = temp;
                let reg_in = self.gpr[instr.rs as usize] as u32;
                let result = (reg_in & mask) | (value >> shift);
                self.gpr[instr.rt as usize] = (result as i32) as u64;
            }
            39 => {
                // LWU
                let instr = ITypeInstruction::from_raw(i);
                let base = self.gpr[instr.rs as usize] as i64;
                let offset = (instr.immediate as i16) as i64;
                let addr = (base + offset) as u64;
                match self.read32(bus, addr) {
                    Ok(data) => self.gpr[instr.rt as usize] = data as u64,
                    Err(e) => return Some(e),
                }
            }
            40 => {
                // SB
                let instr = ITypeInstruction::from_raw(i);
                let base = self.gpr[instr.rs as usize] as i64;
                let offset = (instr.immediate as i16) as i64;
                let addr = (base + offset) as u64;
                let data = self.gpr[instr.rt as usize] as u8;
                self.write8(bus, addr, data)?;
            }
            41 => {
                // SH
                let instr = ITypeInstruction::from_raw(i);
                let base = self.gpr[instr.rs as usize] as i64;
                let offset = (instr.immediate as i16) as i64;
                let addr = (base + offset) as u64;
                let data = self.gpr[instr.rt as usize] as u16;
                self.write16(bus, addr, data)?;
            }
            42 => {
                // SWL
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.immediate as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize] + offset;
                let value = self.gpr[instr.rt as usize] as u32;
                let byte_offset = vaddr & 3;

                match byte_offset {
                    0 => {
                        self.write32(bus, vaddr, value)?;
                    }
                    1 => {
                        self.write8(bus, vaddr, (value >> 16) as u8)?;
                        self.write16(bus, vaddr + 1, (value >> 8) as u16)?;
                    }
                    2 => {
                        self.write16(bus, vaddr, (value >> 16) as u16)?;
                    }
                    3 => {
                        self.write8(bus, vaddr, (value >> 24) as u8)?;
                    }
                    _ => unreachable!(),
                }
            }
            43 => {
                // SW
                let instr = ITypeInstruction::from_raw(i);
                let base = self.gpr[instr.rs as usize] as i64;
                let offset = (instr.immediate as i16) as i64;
                let addr = (base + offset) as u64;
                let data = self.gpr[instr.rt as usize] as u32;
                self.write32(bus, addr, data)?;
            }
            44 => {
                // SDL
                if self.reg_size == RegSize::Reg32 {
                    return Some(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.immediate as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize] + offset;
                let value = self.gpr[instr.rt as usize];
                let byte_offset = vaddr & 7;

                match byte_offset {
                    0 => {
                        self.write64(bus, vaddr, value)?;
                    },
                    1 => {
                        self.write8(bus, vaddr, (value >> 56) as u8)?;
                        self.write16(bus, vaddr + 1, (value >> 40) as u16)?;
                        self.write32(bus, vaddr + 3, (value >> 8) as u32)?;
                    },
                    2 => {
                        self.write16(bus, vaddr, (value >> 48) as u16)?;
                        self.write32(bus, vaddr + 2, (value >> 16) as u32)?;
                    }
                    3 => {
                        self.write8(bus, vaddr, (value >> 56) as u8)?;
                        self.write32(bus, vaddr + 1, (value >> 24) as u32)?;
                    }
                    4 => {
                        self.write32(bus, vaddr, (value >> 32) as u32)?;
                    }
                    5 => {
                        self.write8(bus, vaddr, (value >> 56) as u8)?;
                        self.write16(bus, vaddr + 1, (value >> 40) as u16)?;
                    }
                    6 => {
                        self.write16(bus, vaddr, (value >> 48) as u16)?;
                    }
                    7 => {
                        self.write8(bus, vaddr, (value >> 56) as u8)?;
                    }
                    _ => unreachable!(),
                }
            }
            45 => {
                // SDR
                if self.reg_size == RegSize::Reg32 {
                    return Some(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.immediate as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize] + offset;
                let value = self.gpr[instr.rt as usize];
                let byte_offset = vaddr & 7;

                match byte_offset {
                    0 => {
                        self.write64(bus, vaddr, value)?;
                    },
                    1 => {
                        self.write8(bus, vaddr, (value >> 48) as u8)?;
                        self.write16(bus, vaddr + 1, (value >> 32) as u16)?;
                        self.write32(bus, vaddr + 3, value as u32)?;
                    },
                    2 => {
                        self.write16(bus, vaddr, (value >> 40) as u16)?;
                        self.write32(bus, vaddr + 2, value as u32)?;
                    }
                    3 => {
                        self.write8(bus, vaddr, (value >> 32) as u8)?;
                        self.write32(bus, vaddr + 1, value as u32)?;
                    }
                    4 => {
                        self.write32(bus, vaddr, (value >> 32) as u32)?;
                    }
                    5 => {
                        self.write8(bus, vaddr, (value >> 16) as u8)?;
                        self.write16(bus, vaddr + 1, value as u16)?;
                    }
                    6 => {
                        self.write16(bus, vaddr, value as u16)?;
                    }
                    7 => {
                        self.write8(bus, vaddr, value as u8)?;
                    }
                    _ => unreachable!(),
                }
            }
            46 => {
                // SWR
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.immediate as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize] + offset;
                let value = self.gpr[instr.rt as usize] as u32;
                let byte_offset = vaddr & 3;

                match byte_offset {
                    0 => {
                        self.write32(bus, vaddr, value)?;
                    }
                    1 => {
                        self.write8(bus, vaddr, (value >> 8) as u8)?;
                        self.write16(bus, vaddr + 1, value as u16)?;
                    }
                    2 => {
                        self.write16(bus, vaddr, value as u16)?;
                    }
                    3 => {
                        self.write8(bus, vaddr, value as u8)?;
                    }
                    _ => unreachable!(),
                }
            }
            47 => {} // CASH
            48 => {} // LL
            49 => {} // LWC1
            50 => {} // LWC2
            52 => {} // LLD
            53 => {} // LDC1
            54 => {} // LDC2
            55 => {
                // LD
                if self.reg_size == RegSize::Reg32 {
                    return Some(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let base = self.gpr[instr.rs as usize] as i64;
                let offset = (instr.immediate as i16) as i64;
                let addr = (base + offset) as u64;
                match self.read64(bus, addr) {
                    Ok(data) => self.gpr[instr.rt as usize] = data,
                    Err(e) => return Some(e),
                }
            }
            56 => {
                // SC
                let instr = ITypeInstruction::from_raw(i);
                let value = self.gpr[instr.rt as usize] as u32;
                let vaddr = self.gpr[instr.rs as usize] + (instr.immediate as i16) as u64;
                self.gpr[instr.rt as usize] = 0;

                if self.llbit {
                    self.write32(bus, vaddr, value)?;
                }

                self.gpr[instr.rt as usize] = 1;
            }
            57 => {} // SWC1
            58 => {} // SWC2
            60 => {
                // SCD
                if self.reg_size == RegSize::Reg32 {
                    return Some(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let value = self.gpr[instr.rt as usize];
                let vaddr = self.gpr[instr.rs as usize] + (instr.immediate as i16) as u64;
                self.gpr[instr.rt as usize] = 0;

                if self.llbit {
                    self.write64(bus, vaddr, value)?;
                }

                self.gpr[instr.rt as usize] = 1;
            } // SCD
            61 => {} // SDC1
            62 => {} // SDC2
            63 => {
                // SD
                let instr = ITypeInstruction::from_raw(i);
                let base = self.gpr[instr.rs as usize] as i64;
                let offset = (instr.immediate as i16) as i64;
                let addr = (base + offset) as u64;
                let data = self.gpr[instr.rt as usize];
                self.write64(bus, addr, data)?;
            }
            _ => panic!("Unrecogized opcode: {opcode}"),
        }

        self.gpr[Self::ZR] = 0;

        None
    }

    #[inline]
    pub const fn translate_vaddr(&mut self, vaddr: u32) -> u32 {
        match vaddr {
            0x00000000..=0x7FFFFFFF => 0, /* KUSEG */ // TODO: User Segment, TLB mapped
            0x80000000..=0x9FFFFFFF => vaddr - 0x80000000, /* KSEG0 */
            0xA0000000..=0xBFFFFFFF => vaddr - 0xA0000000, /* KSEG1 */
            0xC0000000..=0xDFFFFFFF => 0, /* KSSEG */ // TODO: Kernel Supervisor segment, TLB mapped
            0xE0000000..=0xFFFFFFFF => 0, /* KSEG3 */ // TODO: Kernel Segment 3, TLB mapped
        }
    }

    fn read8(&mut self, bus: &mut Bus, vaddr: u64) -> Result<u8, CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            return Ok(unsafe { ptr.read_unaligned() });
        }

        let paddr = self.translate_vaddr(vaddr);

        Ok(bus.read8(paddr))
    }

    fn read16(&mut self, bus: &mut Bus, vaddr: u64) -> Result<u16, CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;

        // Alignment check
        if rose_unlikely(vaddr & 1 != 0) {
            return Err(CpuException::AddressErrorLoad);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            return Ok(unsafe { (ptr as *const u16).read_unaligned().to_be() });
        }

        let paddr = self.translate_vaddr(vaddr);

        Ok(bus.read16(paddr))
    }

    fn read32(&mut self, bus: &mut Bus, vaddr: u64) -> Result<u32, CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;

        // Alignment check
        if rose_unlikely(vaddr & 3 != 0) {
            return Err(CpuException::AddressErrorLoad);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            return Ok(unsafe { (ptr as *const u32).read_unaligned().to_be() });
        }

        let paddr = self.translate_vaddr(vaddr);

        Ok(bus.read32(paddr))
    }

    fn read64(&mut self, bus: &mut Bus, vaddr: u64) -> Result<u64, CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;

        // Alignment check
        if rose_unlikely(vaddr & 7 != 0) {
            return Err(CpuException::AddressErrorLoad);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            return Ok(unsafe { (ptr as *const u64).read_unaligned().to_be() });
        }

        let paddr = self.translate_vaddr(vaddr);

        let hi = bus.read32(paddr) as u64;
        let lo = bus.read32(paddr + 4) as u64;

        Ok(hi << 32 | lo)
    }

    fn write8(&mut self, bus: &mut Bus, vaddr: u64, value: u8) -> Option<CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            unsafe {
                ptr.write_unaligned(value);
            }
            return None;
        }

        let paddr = self.translate_vaddr(vaddr);

        bus.write8(paddr, value);

        None
    }

    fn write16(&mut self, bus: &mut Bus, vaddr: u64, value: u16) -> Option<CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;
        let value = value.to_be();

        // Alignment check
        if rose_unlikely(vaddr & 1 != 0) {
            return Some(CpuException::AddressErrorStore);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            unsafe {
                (ptr as *mut u16).write_unaligned(value);
            }
            return None;
        }

        let paddr = self.translate_vaddr(vaddr);

        bus.write16(paddr, value);

        None
    }

    fn write32(&mut self, bus: &mut Bus, vaddr: u64, value: u32) -> Option<CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;
        let value = value.to_be();

        // Alignment check
        if rose_unlikely(vaddr & 3 != 0) {
            return Some(CpuException::AddressErrorStore);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            unsafe {
                (ptr as *mut u32).write_unaligned(value);
            }
            return None;
        }

        let paddr = self.translate_vaddr(vaddr);

        bus.write32(paddr, value);

        None
    }

    fn write64(&mut self, bus: &mut Bus, vaddr: u64, value: u64) -> Option<CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;
        let value = value.to_be();

        // Alignment check
        if rose_unlikely(vaddr & 7 != 0) {
            return Some(CpuException::AddressErrorStore);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            unsafe {
                (ptr as *mut u64).write_unaligned(value);
            }
            return None;
        }

        let paddr = self.translate_vaddr(vaddr);

        bus.write32(paddr, (value >> 32) as u32);
        bus.write32(paddr, value as u32);

        None
    }

    pub fn get_last_exception(&mut self) -> Option<CpuException> {
        self.last_exception
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        memory::bus::Bus,
        processors::vr4300::{CpuException, CpuVR4300},
    };

    const RDRAM_BASE: u64 = 0xFFFFFFFF_80000000;

    fn test_rom() -> Vec<u8> {
        use crate::common::consts::MB;

        vec![0; 8 * MB]
    }

    #[test]
    fn test_unaligned_address_exceptions() {
        let mut cpu = CpuVR4300::new();
        let mut bus = Bus::new(test_rom()).unwrap();

        bus.memory.rdram.0[0] = 0x18;
        bus.memory.rdram.0[1] = 0x19;
        bus.memory.rdram.0[2] = 0x20;
        bus.memory.rdram.0[3] = 0x21;

        bus.memory.rdram.0[4] = 0x78;
        bus.memory.rdram.0[5] = 0x79;
        bus.memory.rdram.0[6] = 0x80;
        bus.memory.rdram.0[7] = 0x81;

        // Happy paths, should throw no exceptions
        assert_eq!(cpu.read8(&mut bus, RDRAM_BASE), Ok(0x18));
        assert_eq!(cpu.read8(&mut bus, RDRAM_BASE + 1), Ok(0x19));
        assert_eq!(cpu.read8(&mut bus, RDRAM_BASE + 2), Ok(0x20));
        assert_eq!(cpu.read8(&mut bus, RDRAM_BASE + 3), Ok(0x21));

        assert_eq!(cpu.read16(&mut bus, RDRAM_BASE), Ok(0x1819));
        assert_eq!(cpu.read16(&mut bus, RDRAM_BASE + 2), Ok(0x2021));

        assert_eq!(cpu.read32(&mut bus, RDRAM_BASE), Ok(0x18192021));

        assert_eq!(
            cpu.read64(&mut bus, RDRAM_BASE),
            Ok(0x18192021_78798081)
        );

        assert_eq!(cpu.write8(&mut bus, RDRAM_BASE, 0x00), None);
        assert_eq!(cpu.write8(&mut bus, RDRAM_BASE + 1, 0x00), None);
        assert_eq!(cpu.write8(&mut bus, RDRAM_BASE + 2, 0x00), None);
        assert_eq!(cpu.write8(&mut bus, RDRAM_BASE + 3, 0x00), None);

        assert_eq!(cpu.write16(&mut bus, RDRAM_BASE, 0x00), None);
        assert_eq!(cpu.write16(&mut bus, RDRAM_BASE + 2, 0x00), None);

        assert_eq!(cpu.write32(&mut bus, RDRAM_BASE, 0x00), None);

        assert_eq!(cpu.write64(&mut bus, RDRAM_BASE, 0x00), None);

        // Unhappy paths, should throw AddressErrorLoad/Store
        assert_eq!(
            cpu.read16(&mut bus, RDRAM_BASE + 1),
            Err(CpuException::AddressErrorLoad)
        );
        assert_eq!(
            cpu.read16(&mut bus, RDRAM_BASE + 3),
            Err(CpuException::AddressErrorLoad)
        );

        assert_eq!(
            cpu.read32(&mut bus, RDRAM_BASE + 1),
            Err(CpuException::AddressErrorLoad)
        );
        assert_eq!(
            cpu.read32(&mut bus, RDRAM_BASE + 2),
            Err(CpuException::AddressErrorLoad)
        );
        assert_eq!(
            cpu.read32(&mut bus, RDRAM_BASE + 3),
            Err(CpuException::AddressErrorLoad)
        );

        assert_eq!(
            cpu.read64(&mut bus, RDRAM_BASE + 1),
            Err(CpuException::AddressErrorLoad)
        );
        assert_eq!(
            cpu.read64(&mut bus, RDRAM_BASE + 2),
            Err(CpuException::AddressErrorLoad)
        );
        assert_eq!(
            cpu.read64(&mut bus, RDRAM_BASE + 3),
            Err(CpuException::AddressErrorLoad)
        );
        assert_eq!(
            cpu.read64(&mut bus, RDRAM_BASE + 4),
            Err(CpuException::AddressErrorLoad)
        );
        assert_eq!(
            cpu.read64(&mut bus, RDRAM_BASE + 5),
            Err(CpuException::AddressErrorLoad)
        );
        assert_eq!(
            cpu.read64(&mut bus, RDRAM_BASE + 6),
            Err(CpuException::AddressErrorLoad)
        );
        assert_eq!(
            cpu.read64(&mut bus, RDRAM_BASE + 7),
            Err(CpuException::AddressErrorLoad)
        );

        assert_eq!(
            cpu.write16(&mut bus, RDRAM_BASE + 1, 0x01),
            Some(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write16(&mut bus, RDRAM_BASE + 3, 0x01),
            Some(CpuException::AddressErrorStore)
        );

        assert_eq!(
            cpu.write32(&mut bus, RDRAM_BASE + 1, 0x01),
            Some(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write32(&mut bus, RDRAM_BASE + 2, 0x01),
            Some(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write32(&mut bus, RDRAM_BASE + 3, 0x01),
            Some(CpuException::AddressErrorStore)
        );

        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 1, 0x01),
            Some(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 2, 0x01),
            Some(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 3, 0x01),
            Some(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 4, 0x01),
            Some(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 5, 0x01),
            Some(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 6, 0x01),
            Some(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 7, 0x01),
            Some(CpuException::AddressErrorStore)
        );

        // Writes that throw exceptions should not go through
        assert_eq!(bus.memory.rdram.0[1], 0x00);
    }

    #[test]
    fn test_read_write_sanity() {
        let mut cpu = CpuVR4300::new();
        let mut bus = Bus::new(test_rom()).unwrap();

        let _ = cpu.write16(&mut bus, RDRAM_BASE, 0x1122);
        assert_eq!(cpu.read16(&mut bus, RDRAM_BASE), Ok(0x1122));

        let _ = cpu.write32(&mut bus, RDRAM_BASE, 0x33445566);
        assert_eq!(cpu.read32(&mut bus, RDRAM_BASE), Ok(0x33445566));

        let _ = cpu.write64(&mut bus, RDRAM_BASE, 0x778899AA_BBCCDDEE);
        assert_eq!(cpu.read64(&mut bus, RDRAM_BASE), Ok(0x778899AA_BBCCDDEE));
    }
}
