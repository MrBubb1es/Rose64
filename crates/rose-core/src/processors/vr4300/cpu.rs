//! -----------------------------------------------------------------------
//! vr4300/mod.rs: Main CPU interpreter implementation
//!
//! A basic implementation of the N64's main CPU. This is a cycle-accurate
//! interpreter implementation: slow, but correct.
//!
//! Author(s): Logan Preston, MrBubblezsz
//! -----------------------------------------------------------------------

use crate::processors::vr4300::CpuException::IntegerOverflow;

/// Representation of the N64's VR4300 processor.
#[repr(C)] // Stable layout needed so JIT code can index fields by offset
pub struct CpuVR4300 {
    pub regs: [u64; 32],
    pub fp_regs: [f64; 32],
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
}

impl CpuVR4300 {
    /// Index of the Zero Register in the regs array
    pub const ZR: usize = 0;
    /// Index of the Link Register in the regs array
    pub const LR: usize = 31;

    pub fn new() -> CpuVR4300 {
        CpuVR4300 {
            regs: [0; 32],
            fp_regs: [0.0; 32],
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
                0 => {}  // SLL
                2 => {}  // SRL
                3 => {}  // SRA
                4 => {}  // SLLV
                6 => {}  // SRLV
                7 => {}  // SRAV
                8 => {}  // JR
                9 => {}  // JALR
                12 => {} // SYSCALL
                13 => {} // BRK
                15 => {} // SYNC
                16 => {} // MFHI
                17 => {} // MTHI
                18 => {} // MFLO
                19 => {} // MTLO
                20 => {} // DSLLV
                22 => {} // DSRLV
                23 => {} // DSRAV
                24 => {} // MULT
                25 => {} // MULTU
                26 => {} // DIV
                27 => {} // DIVU
                28 => {} // DMULT
                29 => {} // DMULTU
                30 => {} // DDIV
                31 => {} // DDIVU
                32 => {
                    // ADD
                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.regs[instr.rs as usize] as i32;
                    let rt = self.regs[instr.rt as usize] as i32;
                    let (sum, overflow) = rs.overflowing_add(rt);

                    let result = match self.reg_size {
                        RegSize::Reg32 => (sum as u32) as u64,
                        RegSize::Reg64 => (sum as i64) as u64,
                    };

                    if overflow {
                        self.raise_exception(CpuException::IntegerOverflow);
                    } else {
                        self.regs[instr.rd as usize] = result;
                    }
                }
                33 => {
                    // ADDU
                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.regs[instr.rs as usize] as i32;
                    let rt = self.regs[instr.rt as usize] as i32;
                    let sum = rs.wrapping_add(rt);

                    let result = match self.reg_size {
                        RegSize::Reg32 => (sum as u32) as u64,
                        RegSize::Reg64 => (sum as i64) as u64,
                    };

                    self.regs[instr.rd as usize] = result;
                }
                34 => {} // SUB
                35 => {} // SUBU
                36 => {
                    // AND
                    let instr = RTypeInstruction::from_raw(i);

                    let mask = match self.reg_size {
                        RegSize::Reg32 => 0xFFFFFFFF_00000000,
                        RegSize::Reg64 => 0xFFFFFFFF_FFFFFFFF,
                    };

                    let result =
                        (self.regs[instr.rt as usize] & self.regs[instr.rs as usize]) & mask;

                    self.regs[instr.rd as usize] = result;
                }
                37 => {
                    // OR
                    let instr = RTypeInstruction::from_raw(i);

                    let mask = match self.reg_size {
                        RegSize::Reg32 => 0xFFFFFFFF_00000000,
                        RegSize::Reg64 => 0xFFFFFFFF_FFFFFFFF,
                    };

                    let result =
                        (self.regs[instr.rt as usize] | self.regs[instr.rs as usize]) & mask;

                    self.regs[instr.rd as usize] = result;
                }
                38 => {
                    // XOR
                    let instr = RTypeInstruction::from_raw(i);

                    let mask = match self.reg_size {
                        RegSize::Reg32 => 0xFFFFFFFF_00000000,
                        RegSize::Reg64 => 0xFFFFFFFF_FFFFFFFF,
                    };

                    let result =
                        (self.regs[instr.rt as usize] ^ self.regs[instr.rs as usize]) & mask;

                    self.regs[instr.rd as usize] = result;
                }
                39 => {
                    // NOR
                    let instr = RTypeInstruction::from_raw(i);

                    let mask = match self.reg_size {
                        RegSize::Reg32 => 0xFFFFFFFF_00000000,
                        RegSize::Reg64 => 0xFFFFFFFF_FFFFFFFF,
                    };

                    let result =
                        (!(self.regs[instr.rt as usize] | self.regs[instr.rs as usize])) & mask;

                    self.regs[instr.rd as usize] = result;
                }
                42 => {} // SLT
                43 => {} // SLTU
                44 => {} // DADD
                45 => {} // DADDU
                46 => {} // DSUB
                47 => {} // DSUBU
                48 => {} // TGE
                49 => {} // TGEU
                50 => {} // TLT
                51 => {} // TLTU
                52 => {} // TEQ
                54 => {} // TNE
                56 => {} // DSLL
                58 => {} // DSRL
                59 => {} // DSRA
                60 => {} // DSLL32
                62 => {} // DSRL32
                63 => {} // DSRA32
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

                let rs = self.regs[instr.rs as usize] as i32;
                let immediate = (instr.immediate as i16) as i32;
                let (sum, overflow) = rs.overflowing_add(immediate);

                let result = match self.reg_size {
                    RegSize::Reg32 => (sum as u32) as u64,
                    RegSize::Reg64 => (sum as i64) as u64,
                };

                if overflow {
                    self.raise_exception(IntegerOverflow);
                } else {
                    self.regs[instr.rt as usize] = result;
                }
            }
            9 => {
                // ADDIU
                let instr = ITypeInstruction::from_raw(i);

                let rs = self.regs[instr.rs as usize] as i32;
                let immediate = (instr.immediate as i16) as i32;
                let sum = rs.wrapping_add(immediate);

                let result = match self.reg_size {
                    RegSize::Reg32 => (sum as u32) as u64,
                    RegSize::Reg64 => (sum as i64) as u64,
                };

                self.regs[instr.rt as usize] = result;
            }
            10 => {} // SLTI
            11 => {} // SLTIU
            12 => {} // ANDI
            13 => {} // ORI
            14 => {} // XORI
            15 => {} // LUI
            16 => {} // COP0
            17 => {} // COP1
            18 => {} // COP2
            20 => {} // BEQL
            21 => {} // BNEL
            22 => {} // BLEZL
            23 => {} // BGTZL
            24 => {} // DADDI
            25 => {} // DADDIU
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
    }

    fn read(&mut self, _addr: u64) -> u32 {
        todo!("Cpu reads");
    }

    fn write(&mut self, _addr: u64, _val: u32) {
        todo!("Cpu writes");
    }

    fn raise_exception(&mut self, e: CpuException) {
        self.last_exception = Some(e);
    }

    pub fn get_last_exception(&mut self) -> Option<CpuException> {
        self.last_exception
    }
}
