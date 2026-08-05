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
    processors::vr4300::CpuException::IntegerOverflow,
};

/// Representation of the N64's VR4300 processor.
#[repr(C)] // Stable layout needed so JIT code can index fields by offset
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RegSize {
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
        CpuVR4300 {
            gpr: [0; 32],
            fpr: [0.0; 32],
            pc: 0,
            mult_hi: 0,
            mult_lo: 0,
            fp_control: 0.0,
            fp_revision: 0.0,
            llbit: false,
            reg_size: RegSize::Reg32,
            last_exception: None,
        }
    }

    pub fn execute_instruction(&mut self, i: u32) {
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
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);
                            let rt = self.gpr[instr.rt as usize];
                            let shift = self.gpr[instr.rs as usize] & 0x3F;
                            let result = rt << shift;
                            self.gpr[instr.rd as usize] = result;
                        }
                    }
                }
                22 => {
                    // DSRLV
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);
                            let rt = self.gpr[instr.rt as usize];
                            let shift = self.gpr[instr.rs as usize] & 0x3F;
                            let result = rt >> shift;
                            self.gpr[instr.rd as usize] = result;
                        }
                    }
                }
                23 => {
                    // DSRAV
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);
                            // Cast to signed for arithmetic shift
                            let rt = self.gpr[instr.rt as usize] as i64;
                            let shift = self.gpr[instr.rs as usize] & 0x3F;
                            let result = rt >> shift;
                            self.gpr[instr.rd as usize] = result as u64;
                        }
                    }
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
                    // Assuming 32 and 64 bit versions work identically WRT sign extension for now
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;
                    let q = rs.checked_div(rt).unwrap_or(if rs < 0 {1} else {-1});
                    let r = rs.checked_rem(rt).unwrap_or(rs);

                    self.mult_lo = (q as i32) as u64;
                    self.mult_hi = (r as i32) as u64;
                }
                27 => {
                    // DIVU
                    // Assuming 32 and 64 bit versions work identically WRT sign extension for now
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize];
                    let rt = self.gpr[instr.rt as usize];
                    let q = rs.checked_div(rt).unwrap_or(u64::MAX);
                    let r = rs.checked_rem(rt).unwrap_or(rs);

                    self.mult_lo = (q as i32) as u64;
                    self.mult_hi = (r as i32) as u64;
                }
                28 => {
                    // DMULT
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);
                            let rs = (self.gpr[instr.rs as usize] as i64) as i128;
                            let rt = (self.gpr[instr.rt as usize] as i64) as i128;
                            let prod = rs * rt;
                            self.mult_lo = prod as u64;
                            self.mult_hi = (prod >> 64) as u64;
                        }
                    }
                }
                29 => {
                    // DMULTU
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);
                            let rs = self.gpr[instr.rs as usize] as u128;
                            let rt = self.gpr[instr.rt as usize] as u128;
                            let prod = rs * rt;
                            self.mult_lo = prod as u64;
                            self.mult_hi = (prod >> 64) as u64;
                        }
                    }
                }
                30 => {
                    // DDIV
                    // Assuming 32 and 64 bit versions work identically WRT sign extension for now
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;
                    let q = rs.checked_div(rt).unwrap_or(if rs < 0 {1} else {-1});
                    let r = rs.checked_rem(rt).unwrap_or(rs);

                    self.mult_lo = q as u64;
                    self.mult_hi = r as u64;
                }
                31 => {
                    // DDIVU
                    // Assuming 32 and 64 bit versions work identically WRT sign extension for now
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
                        self.raise_exception(CpuException::IntegerOverflow);
                    } else {
                        self.gpr[instr.rd as usize] = sum as u64;
                    }
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
                        self.raise_exception(CpuException::IntegerOverflow);
                    } else {
                        self.gpr[instr.rd as usize] = diff as u64;
                    }
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
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);

                            let rs = self.gpr[instr.rs as usize] as i64;
                            let rt = self.gpr[instr.rt as usize] as i64;

                            let (result, overflow) = rs.overflowing_add(rt);

                            if overflow {
                                self.raise_exception(CpuException::IntegerOverflow);
                            } else {
                                self.gpr[instr.rd as usize] = result as u64;
                            }
                        }
                    }
                }
                45 => {
                    // DADDU
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);

                            let rs = self.gpr[instr.rs as usize];
                            let rt = self.gpr[instr.rt as usize];

                            let result = rs.wrapping_add(rt);

                            self.gpr[instr.rd as usize] = result;
                        }
                    }
                }
                46 => {
                    // DSUB
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);

                            let rs = self.gpr[instr.rs as usize] as i64;
                            let rt = self.gpr[instr.rt as usize] as i64;

                            let (result, overflow) = rs.overflowing_sub(rt);

                            if overflow {
                                self.raise_exception(CpuException::IntegerOverflow);
                            } else {
                                self.gpr[instr.rd as usize] = result as u64;
                            }
                        }
                    }
                }
                47 => {
                    // DSUBU
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);

                            let rs = self.gpr[instr.rs as usize];
                            let rt = self.gpr[instr.rt as usize];

                            let result = rs.wrapping_sub(rt);

                            self.gpr[instr.rd as usize] = result;
                        }
                    }
                }
                48 => {} // TGE
                49 => {} // TGEU
                50 => {} // TLT
                51 => {} // TLTU
                52 => {} // TEQ
                54 => {} // TNE
                56 => {
                    // DSLL
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);
                            let rt = self.gpr[instr.rt as usize];
                            let result = rt << instr.shift;
                            self.gpr[instr.rd as usize] = result;
                        }
                    }
                }
                58 => {
                    // DSRL
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);
                            let rt = self.gpr[instr.rt as usize];
                            let result = rt >> instr.shift;
                            self.gpr[instr.rd as usize] = result;
                        }
                    }
                }
                59 => {
                    // DSRA
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);
                            // Cast to signed value for arithmetic shift.
                            let rt = self.gpr[instr.rt as usize] as i64;
                            let result = rt >> instr.shift;
                            self.gpr[instr.rd as usize] = result as u64;
                        }
                    }
                }
                60 => {
                    // DSLL32
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);
                            let rt = self.gpr[instr.rt as usize];
                            let result = rt << (instr.shift + 32);
                            self.gpr[instr.rd as usize] = result;
                        }
                    }
                }
                62 => {
                    // DSRL32
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);
                            let rt = self.gpr[instr.rt as usize];
                            let result = rt >> (instr.shift + 32);
                            self.gpr[instr.rd as usize] = result;
                        }
                    }
                }
                63 => {
                    // DSRA32
                    match self.reg_size {
                        RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                        RegSize::Reg64 => {
                            let instr = RTypeInstruction::from_raw(i);
                            // Cast to signed for arithmetic shift
                            let rt = self.gpr[instr.rt as usize] as i64;
                            let result = rt >> (instr.shift + 32);
                            self.gpr[instr.rd as usize] = result as u64;
                        }
                    }
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
                    self.raise_exception(IntegerOverflow);
                } else {
                    self.gpr[instr.rt as usize] = sum as u64;
                }
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
            15 => {} // LUI
            16 => {} // COP0
            17 => {} // COP1
            18 => {} // COP2
            20 => {} // BEQL
            21 => {} // BNEL
            22 => {} // BLEZL
            23 => {} // BGTZL
            24 => {
                // DADDI
                match self.reg_size {
                    RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                    RegSize::Reg64 => {
                        let instr = ITypeInstruction::from_raw(i);
                        let rs = self.gpr[instr.rs as usize] as i64;
                        let immediate = (instr.immediate as i16) as i64;
                        let (result, overflow) = rs.overflowing_add(immediate);
                        if overflow {
                            self.raise_exception(CpuException::IntegerOverflow);
                        } else {
                            self.gpr[instr.rt as usize] = result as u64;
                        }
                    }
                }
            }
            25 => {
                // DADDIU
                match self.reg_size {
                    RegSize::Reg32 => self.raise_exception(CpuException::ReservedInstruction),
                    RegSize::Reg64 => {
                        let instr = ITypeInstruction::from_raw(i);
                        let rs = self.gpr[instr.rs as usize];
                        let immediate = (instr.immediate as i16) as u64;
                        let result = rs.wrapping_add(immediate);
                        self.gpr[instr.rt as usize] = result;
                    }
                }
            }
            26 => {} // LDL
            27 => {} // LDR
            32 => {} // LB
            33 => {} // LH
            34 => {} // LWL
            35 => {} // LW
            36 => {} // LBU
            37 => {} // LHU
            38 => {} // LWR
            39 => {} // LWU
            40 => {} // SB
            41 => {} // SH
            42 => {} // SWL
            43 => {} // SD
            44 => {} // SDL
            45 => {} // SDR
            46 => {} // SWR
            47 => {} // CASH
            48 => {} // LL
            49 => {} // LWC1
            50 => {} // LWC2
            52 => {} // LLD
            53 => {} // LDC1
            54 => {} // LDC2
            55 => {} // LD
            56 => {} // SC
            57 => {} // SWC1
            58 => {} // SWC2
            60 => {} // SCD
            61 => {} // SDC1
            62 => {} // SDC2
            63 => {} // SD
            _ => panic!("Unrecogized opcode: {opcode}"),
        }

        self.gpr[Self::ZR] = 0;
    }

    #[inline]
    pub const fn translate_vaddr(&mut self, vaddr: u64) -> u32 {
        let va = vaddr as u32;

        match va {
            0x00000000..=0x7FFFFFFF => 0, /* KUSEG */ // TODO: User Segment, TLB mapped
            0x80000000..=0x9FFFFFFF => va - 0x80000000, /* KSEG0 */
            0xA0000000..=0xBFFFFFFF => va - 0xA0000000, /* KSEG1 */
            0xC0000000..=0xDFFFFFFF => 0, /* KSSEG */ // TODO: Kernel Supervisor segment, TLB mapped
            0xE0000000..=0xFFFFFFFF => 0, /* KSEG3 */ // TODO: Kernel Segment 3, TLB mapped
        }
    }

    fn read8(&mut self, bus: &mut Bus, vaddr: u64) -> Result<u8, CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let va = vaddr as u32;

        if let Some(ptr) = bus.memory.get_raw_mem(va) {
            return Ok(unsafe { ptr.read_unaligned() });
        }

        let paddr = self.translate_vaddr(vaddr);

        Ok(bus.read8(paddr))
    }

    fn read16(&mut self, bus: &mut Bus, vaddr: u64) -> Result<u16, CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let va = vaddr as u32;

        // Alignment check
        if rose_unlikely(va & 1 != 0) {
            return Err(CpuException::AddressErrorLoad);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(va) {
            return Ok(unsafe { (ptr as *const u16).read_unaligned() });
        }

        let paddr = self.translate_vaddr(vaddr);

        Ok(bus.read16(paddr))
    }

    fn read32(&mut self, bus: &mut Bus, vaddr: u64) -> Result<u32, CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let va = vaddr as u32;

        // Alignment check
        if rose_unlikely(va & 3 != 0) {
            return Err(CpuException::AddressErrorLoad);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(va) {
            return Ok(unsafe { (ptr as *const u32).read_unaligned() });
        }

        let paddr = self.translate_vaddr(vaddr);

        Ok(bus.read32(paddr))
    }

    fn read64(&mut self, bus: &mut Bus, vaddr: u64) -> Result<u64, CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let hi = self.read32(bus, vaddr)? as u64;
        let lo = self.read32(bus, vaddr + 4)? as u64;
        Ok(hi << 32 | lo)
    }

    fn write8(&mut self, bus: &mut Bus, vaddr: u64, value: u8) -> Option<CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let va = vaddr as u32;

        if let Some(ptr) = bus.memory.get_raw_mem(va) {
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

        let va = vaddr as u32;

        // Alignment check
        if rose_unlikely(va & 1 != 0) {
            return Some(CpuException::AddressErrorStore);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(va) {
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

        let va = vaddr as u32;

        // Alignment check
        if rose_unlikely(va & 3 != 0) {
            return Some(CpuException::AddressErrorStore);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(va) {
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

        self.write32(bus, vaddr, (value >> 32) as u32)?;
        self.write32(bus, vaddr + 4, value as u32)?;
        None
    }

    fn raise_exception(&mut self, e: CpuException) {
        self.last_exception = Some(e);
    }

    pub fn get_last_exception(&mut self) -> Option<CpuException> {
        self.last_exception
    }
}
