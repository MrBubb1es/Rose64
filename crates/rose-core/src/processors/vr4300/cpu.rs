//! -----------------------------------------------------------------------
//! vr4300/cpu.rs: Main CPU interpreter implementation
//!
//! A basic implementation of the N64's main CPU. This is a cycle-accurate
//! interpreter implementation: slow, but correct.
//!
//! Author(s): MrBubblezsz, logocrazymon
//! -----------------------------------------------------------------------

use crate::{
    common::hint::{rose_likely, rose_unlikely},
    memory::bus::{Bus, MemoryAccess},
    processors::vr4300::{
        cp0::Cp0,
        instructions::{ITypeInstruction, JTypeInstruction, RTypeInstruction},
    },
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

    pub cp0: Cp0,
    exec_state: ExecutionState,
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
pub enum RegSize {
    #[default]
    Reg32,
    Reg64,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum CpuException {
    Interrupt,
    TlbModification,
    TlbMissLoad,
    TlbMissStore,
    AddressErrorLoad,
    AddressErrorStore,
    BusErrorInstrFetch,
    BusErrorLoadStore,
    Syscall,
    Breakpoint,
    ReservedInstruction,
    CoprocessorUnusable,
    ArithmeticOverflow,
    Trap,
    FloatingPoint,
    Watch,
}

#[derive(Debug, PartialEq, Clone, Copy, Default)]
enum ExecutionState {
    #[default]
    Normal,
    /// Jumping and branching instructions move to this execution state. This
    /// is an intermediate state that is immediately set to the Delay state
    /// at the end of the instruction.
    Jump { addr: u64, taken: bool },
    /// The state the CPU is in when executing a delay slot instruction. This
    /// affects the execution of some instructions (namely jumps and branches,
    /// see ASSUMPTIONS.md), and causes the delay bit to be set in CP0 when
    /// an exception occurs.
    Delay { addr: u64, taken: bool },
}

impl CpuVR4300 {
    /// Index of the Zero Register in the regs array
    pub const ZR: usize = 0;
    /// Index of the Link Register in the regs array
    pub const LR: usize = 31;

    pub fn new() -> CpuVR4300 {
        CpuVR4300::default()
    }

    pub fn in_branch_delay_slot(&self) -> bool {
        matches!(self.exec_state, ExecutionState::Delay { .. })
    }

    pub fn execute_instruction(&mut self, bus: &mut Bus, i: u32) -> Result<(), CpuException> {
        assert!(!matches!(self.exec_state, ExecutionState::Jump { .. }));

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
                8 => {
                    // JR
                    if rose_likely(!self.in_branch_delay_slot()) {
                        let instr = RTypeInstruction::from_raw(i);
                        let rs = self.gpr[instr.rs as usize];
                        self.exec_state = ExecutionState::Jump {
                            addr: rs,
                            taken: true,
                        };
                    }
                }
                9 => {
                    // JALR
                    if rose_likely(!self.in_branch_delay_slot()) {
                        let instr = RTypeInstruction::from_raw(i);
                        let rs = self.gpr[instr.rs as usize];
                        self.gpr[instr.rd as usize] = self.pc.wrapping_add(8);
                        self.exec_state = ExecutionState::Jump {
                            addr: rs,
                            taken: true,
                        };
                    }
                }
                12 => {
                    // SYSCALL
                    return Err(CpuException::Syscall);
                }
                13 => {
                    // BRK
                    return Err(CpuException::Breakpoint);
                }
                15 => {
                    // SYNC: is a NOP on the VR4300i
                }
                16 => {
                    // MFHI
                    let instr = RTypeInstruction::from_raw(i);
                    self.gpr[instr.rd as usize] = self.mult_hi;
                }
                17 => {
                    // MTHI
                    let instr = RTypeInstruction::from_raw(i);
                    self.mult_hi = self.gpr[instr.rs as usize];
                }
                18 => {
                    // MFLO
                    let instr = RTypeInstruction::from_raw(i);
                    self.gpr[instr.rd as usize] = self.mult_lo;
                }
                19 => {
                    // MTLO
                    let instr = RTypeInstruction::from_raw(i);
                    self.mult_lo = self.gpr[instr.rs as usize];
                }
                20 => {
                    // DSLLV
                    if self.reg_size == RegSize::Reg32 {
                        return Err(CpuException::ReservedInstruction);
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
                        return Err(CpuException::ReservedInstruction);
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
                        return Err(CpuException::ReservedInstruction);
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
                    //
                    // MULT HARDWARE BUG:
                    //   Acts as a 64-bit by 35-bit signed multiplication,
                    //   affecting results when registers are not properly sign
                    //   extended values. See:
                    //   https://n64brew.dev/wiki/VR4300#Sign_extension_bugs
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;
                    let rt_sign_ext = (rt << 29) >> 29;
                    let prod = rs.wrapping_mul(rt_sign_ext);
                    self.mult_lo = (prod as i32) as u64;
                    self.mult_hi = (prod >> 32) as u64;
                }
                25 => {
                    // MULTU
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as u32 as u64;
                    let rt = self.gpr[instr.rt as usize] as u32 as u64;
                    let prod = rs * rt;
                    self.mult_lo = (prod as i32) as u64;
                    self.mult_hi = ((prod >> 32) as i32) as u64;
                }
                26 => {
                    // DIV
                    // 
                    // DIV HARDWARE BUG:
                    //   Acts as a 32-bit by 35-bit signed division, affecting
                    //   results when registers are not properly sign extended
                    //   values. Additionally, if bits 63 and 32 of $rt differ,
                    //   then the quotient is an unknown incorrect value, and
                    //   the remainder is calculated via `rs - q*rt`. We are not
                    //   modeling the behavior in this case, as there is no
                    //   source documenting what the output should be. See:
                    //   https://n64brew.dev/wiki/VR4300#Sign_extension_bugs
                    //
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as i32 as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;
                    let rt_sign_ext = (rt << 29) >> 29;

                    let (q, r) = if rt == 0 {
                        (
                            if rs < 0 { 1 } else { -1 },
                            rs,
                        )
                    } else {
                        (rs / rt_sign_ext, rs % rt_sign_ext)
                    };

                    self.mult_lo = (q as i32) as u64;
                    self.mult_hi = (r as i32) as u64;
                }
                27 => {
                    // DIVU
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as u32;
                    let rt = self.gpr[instr.rt as usize] as u32;

                    // TODO: Maybe rose_unlikely
                    #[allow(clippy::manual_checked_ops)]
                    let (q, r) = if rt == 0 {
                        (u32::MAX, rs)
                    } else {
                        (rs / rt, rs % rt)
                    };

                    self.mult_lo = (q as i32) as u64;
                    self.mult_hi = (r as i32) as u64;
                }
                28 => {
                    // DMULT
                    if self.reg_size == RegSize::Reg32 {
                        return Err(CpuException::ReservedInstruction);
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
                        return Err(CpuException::ReservedInstruction);
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
                    let rs = self.gpr[instr.rs as usize] as i64 as i128;
                    let rt = self.gpr[instr.rt as usize] as i64 as i128;

                    // TODO: Maybe rose_unlikely
                    let (q, r) = if rt == 0 {
                        (
                            if rs < 0 { 1 } else { -1 },
                            rs as i64,
                        )
                    } else {
                        ((rs / rt) as i64, (rs % rt) as i64)
                    };

                    self.mult_lo = q as u64;
                    self.mult_hi = r as u64;
                }
                31 => {
                    // DDIVU
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize];
                    let rt = self.gpr[instr.rt as usize];

                    // TODO: Maybe rose_unlikely
                    #[allow(clippy::manual_checked_ops)]
                    let (q, r) = if rt == 0 {
                        (u64::MAX, rs)
                    } else {
                        (rs / rt, rs % rt)
                    };

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
                        return Err(CpuException::ArithmeticOverflow);
                    }

                    self.gpr[instr.rd as usize] = sum as u64;
                }
                33 => {
                    // ADDU
                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize] as u32;
                    let rt = self.gpr[instr.rt as usize] as u32;
                    let sum = rs.wrapping_add(rt);

                    self.gpr[instr.rd as usize] = sum as i32 as u64;
                }
                34 => {
                    // SUB
                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize] as i32;
                    let rt = self.gpr[instr.rt as usize] as i32;
                    let (diff, overflow) = rs.overflowing_sub(rt);

                    if overflow {
                        return Err(CpuException::ArithmeticOverflow);
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
                        return Err(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize] as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;

                    let (result, overflow) = rs.overflowing_add(rt);

                    if overflow {
                        return Err(CpuException::ArithmeticOverflow);
                    }

                    self.gpr[instr.rd as usize] = result as u64;
                }
                45 => {
                    // DADDU
                    if self.reg_size == RegSize::Reg32 {
                        return Err(CpuException::ReservedInstruction);
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
                        return Err(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize] as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;

                    let (result, overflow) = rs.overflowing_sub(rt);

                    if overflow {
                        return Err(CpuException::ArithmeticOverflow);
                    }

                    self.gpr[instr.rd as usize] = result as u64;
                }
                47 => {
                    // DSUBU
                    if self.reg_size == RegSize::Reg32 {
                        return Err(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);

                    let rs = self.gpr[instr.rs as usize];
                    let rt = self.gpr[instr.rt as usize];

                    let result = rs.wrapping_sub(rt);

                    self.gpr[instr.rd as usize] = result;
                }
                48 => {
                    // TGE
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;
                    if rs >= rt {
                        return Err(CpuException::Trap);
                    }
                }
                49 => {
                    // TGEU
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize];
                    let rt = self.gpr[instr.rt as usize];
                    if rs >= rt {
                        return Err(CpuException::Trap);
                    }
                }
                50 => {
                    // TLT
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize] as i64;
                    let rt = self.gpr[instr.rt as usize] as i64;
                    if rs < rt {
                        return Err(CpuException::Trap);
                    }
                }
                51 => {
                    // TLTU
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize];
                    let rt = self.gpr[instr.rt as usize];
                    if rs < rt {
                        return Err(CpuException::Trap);
                    }
                }
                52 => {
                    // TEQ
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize];
                    let rt = self.gpr[instr.rt as usize];
                    if rs == rt {
                        return Err(CpuException::Trap);
                    }
                }
                54 => {
                    // TNE
                    let instr = RTypeInstruction::from_raw(i);
                    let rs = self.gpr[instr.rs as usize];
                    let rt = self.gpr[instr.rt as usize];
                    if rs != rt {
                        return Err(CpuException::Trap);
                    }
                }
                56 => {
                    // DSLL
                    if self.reg_size == RegSize::Reg32 {
                        return Err(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let result = rt << instr.shift;
                    self.gpr[instr.rd as usize] = result;
                }
                58 => {
                    // DSRL
                    if self.reg_size == RegSize::Reg32 {
                        return Err(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let result = rt >> instr.shift;
                    self.gpr[instr.rd as usize] = result;
                }
                59 => {
                    // DSRA
                    if self.reg_size == RegSize::Reg32 {
                        return Err(CpuException::ReservedInstruction);
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
                        return Err(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let result = rt << (instr.shift + 32);
                    self.gpr[instr.rd as usize] = result;
                }
                62 => {
                    // DSRL32
                    if self.reg_size == RegSize::Reg32 {
                        return Err(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    let rt = self.gpr[instr.rt as usize];
                    let result = rt >> (instr.shift + 32);
                    self.gpr[instr.rd as usize] = result;
                }
                63 => {
                    // DSRA32
                    if self.reg_size == RegSize::Reg32 {
                        return Err(CpuException::ReservedInstruction);
                    }

                    let instr = RTypeInstruction::from_raw(i);
                    // Cast to signed for arithmetic shift
                    let rt = self.gpr[instr.rt as usize] as i64;
                    let result = rt >> (instr.shift + 32);
                    self.gpr[instr.rd as usize] = result as u64;
                }
                _ => panic!("Unrecognized SPECIAL opcode {i}"),
            },
            1 => {
                // REGIMM
                let rt = (i >> 16) & 0x1F;
                match rt {
                    0 => {
                        // BLTZ
                        self.branch_instr(i, |rs: i64, _rt: i64| rs < 0, false);
                    }
                    1 => {
                        // BGEZ
                        self.branch_instr(i, |rs: i64, _rt: i64| rs >= 0, false);
                    }
                    2 => {
                        // BLTZL
                        self.branch_likely_instr(i, |rs: i64, _rt: i64| rs <= 0, false);
                    }
                    3 => {
                        // BGEZL
                        self.branch_likely_instr(i, |rs: i64, _rt: i64| rs >= 0, false);
                    }
                    8 => {
                        // TGEI
                        let instr = ITypeInstruction::from_raw(i);
                        let rs = self.gpr[instr.rs as usize] as i64;
                        let imm = (instr.imm as i16) as i64;
                        if rs >= imm {
                            return Err(CpuException::Trap);
                        }
                    }
                    9 => {
                        // TGEIU
                        let instr = ITypeInstruction::from_raw(i);
                        let rs = self.gpr[instr.rs as usize];
                        let imm = (instr.imm as i16) as u64;
                        if rs >= imm {
                            return Err(CpuException::Trap);
                        }
                    }
                    10 => {
                        // TLTI
                        let instr = ITypeInstruction::from_raw(i);
                        let rs = self.gpr[instr.rs as usize] as i64;
                        let imm = (instr.imm as i16) as i64;
                        if rs < imm {
                            return Err(CpuException::Trap);
                        }
                    }
                    11 => {
                        // TLTIU
                        let instr = ITypeInstruction::from_raw(i);
                        let rs = self.gpr[instr.rs as usize];
                        let imm = (instr.imm as i16) as u64;
                        if rs < imm {
                            return Err(CpuException::Trap);
                        }
                    }
                    12 => {
                        // TEQI
                        let instr = ITypeInstruction::from_raw(i);
                        let rs = self.gpr[instr.rs as usize];
                        let imm = (instr.imm as i16) as u64;
                        if rs == imm {
                            return Err(CpuException::Trap);
                        }
                    }
                    14 => {
                        // TNEI
                        let instr = ITypeInstruction::from_raw(i);
                        let rs = self.gpr[instr.rs as usize];
                        let imm = (instr.imm as i16) as u64;
                        if rs != imm {
                            return Err(CpuException::Trap);
                        }
                    }
                    16 => {
                        // BLTZAL
                        self.branch_instr(i, |rs: i64, _rt: i64| rs < 0, true);
                    }
                    17 => {
                        // BGEZAL
                        self.branch_instr(i, |rs: i64, _rt: i64| rs >= 0, true);
                    }
                    18 => {
                        // BLTZALL
                        self.branch_likely_instr(i, |rs: i64, _rt: i64| rs < 0, true);
                    }
                    19 => {
                        // BGEZALL
                        self.branch_likely_instr(i, |rs: i64, _rt: i64| rs >= 0, true);
                    }
                    _ => {}
                }
            }
            2 => {
                // J
                if rose_likely(!self.in_branch_delay_slot()) {
                    let instr = JTypeInstruction::from_raw(i);
                    let addr_hi = self.pc.wrapping_add(4) & 0xFFFFFFFF_F0000000;
                    let addr_lo = instr.target << 2;
                    let new_addr = addr_hi | (addr_lo as u64);
                    self.exec_state = ExecutionState::Jump {
                        addr: new_addr,
                        taken: true,
                    };
                }
            }
            3 => {
                // JAL
                if rose_likely(!self.in_branch_delay_slot()) {
                    let instr = JTypeInstruction::from_raw(i);
                    let addr_hi = self.pc.wrapping_add(4) & 0xFFFFFFFF_F0000000;
                    let addr_lo = instr.target << 2;
                    let new_addr = addr_hi | (addr_lo as u64);
                    self.gpr[Self::LR] = self.pc.wrapping_add(8);
                    self.exec_state = ExecutionState::Jump {
                        addr: new_addr,
                        taken: true,
                    };
                }
            }
            4 => {
                // BEQ
                self.branch_instr(i, |rs: i64, rt: i64| rs == rt, false);
            }
            5 => {
                // BNE
                self.branch_instr(i, |rs: i64, rt: i64| rs != rt, false);
            }
            6 => {
                // BLEZ
                self.branch_instr(i, |rs: i64, _rt: i64| rs <= 0, false);
            }
            7 => {
                // BGTZ
                self.branch_instr(i, |rs: i64, _rt: i64| rs > 0, false);
            }
            8 => {
                // ADDI
                let instr = ITypeInstruction::from_raw(i);

                let rs = self.gpr[instr.rs as usize] as i32;
                let immediate = (instr.imm as i16) as i32;
                let (sum, overflow) = rs.overflowing_add(immediate);

                if overflow {
                    return Err(CpuException::ArithmeticOverflow);
                }

                self.gpr[instr.rt as usize] = sum as u64;
            }
            9 => {
                // ADDIU
                let instr = ITypeInstruction::from_raw(i);

                let rs = self.gpr[instr.rs as usize] as i32;
                let immediate = (instr.imm as i16) as i32;
                let sum = rs.wrapping_add(immediate);

                self.gpr[instr.rt as usize] = sum as u64;
            }
            10 => {
                // SLTI
                let instr = ITypeInstruction::from_raw(i);
                let rs = self.gpr[instr.rs as usize] as i64;
                let imm = (instr.imm as i16) as i64;
                self.gpr[instr.rt as usize] = if rs < imm { 1u64 } else { 0u64 };
            }
            11 => {
                // SLTIU
                let instr = ITypeInstruction::from_raw(i);
                let rs = self.gpr[instr.rs as usize];
                let imm = instr.imm as u64;
                self.gpr[instr.rt as usize] = if rs < imm { 1u64 } else { 0u64 };
            }
            12 => {
                // ANDI
                let instr = ITypeInstruction::from_raw(i);

                self.gpr[instr.rt as usize] = instr.imm as u64 & self.gpr[instr.rs as usize];
            }
            13 => {
                // ORI
                let instr = ITypeInstruction::from_raw(i);

                self.gpr[instr.rt as usize] = instr.imm as u64 | self.gpr[instr.rs as usize];
            }
            14 => {
                // XORI
                let instr = ITypeInstruction::from_raw(i);

                self.gpr[instr.rt as usize] = instr.imm as u64 ^ self.gpr[instr.rs as usize];
            }
            15 => {
                // LUI
                let instr = ITypeInstruction::from_raw(i);
                let val = ((instr.imm as i32) << 16) as u64;
                self.gpr[instr.rt as usize] = val;
            }
            16 => {
                todo!("Instruction COP0")
            } // COP0
            17 => {
                todo!("Instruction COP1")
            } // COP1
            18 => {
                todo!("Instruction COP2")
            } // COP2
            20 => {
                // BEQL
                self.branch_likely_instr(i, |rs: i64, rt: i64| rs == rt, false);
            }
            21 => {
                // BNEL
                self.branch_likely_instr(i, |rs: i64, rt: i64| rs != rt, false);
            }
            22 => {
                // BLEZL
                self.branch_likely_instr(i, |rs: i64, _rt: i64| rs <= 0, false);
            }
            23 => {
                // BGTZL
                self.branch_likely_instr(i, |rs: i64, _rt: i64| rs > 0, false);
            }
            24 => {
                // DADDI
                if self.reg_size == RegSize::Reg32 {
                    return Err(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let rs = self.gpr[instr.rs as usize] as i64;
                let immediate = (instr.imm as i16) as i64;
                let (result, overflow) = rs.overflowing_add(immediate);

                if overflow {
                    return Err(CpuException::ArithmeticOverflow);
                }

                self.gpr[instr.rt as usize] = result as u64;
            }
            25 => {
                // DADDIU
                if self.reg_size == RegSize::Reg32 {
                    return Err(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let rs = self.gpr[instr.rs as usize];
                let immediate = (instr.imm as i16) as u64;
                let result = rs.wrapping_add(immediate);
                self.gpr[instr.rt as usize] = result;
            }
            26 => {
                // LDL
                if self.reg_size == RegSize::Reg32 {
                    return Err(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let rt = self.gpr[instr.rt as usize];
                let rs = self.gpr[instr.rs as usize];
                let offset = (instr.imm as i16) as u64;
                let vaddr = rs.wrapping_add(offset);
                let vaddr_aligned = vaddr & !7;
                let value = self.read64(bus, vaddr_aligned)?;
                let byte_offset = vaddr & 7;
                let shift = 8 * byte_offset as u32;
                let mask = u64::MAX.checked_shr(64 - shift).unwrap_or(0);
                let result = (value << shift) | (rt & mask);
                self.gpr[instr.rt as usize] = result;
            }
            27 => {
                // LDR
                if self.reg_size == RegSize::Reg32 {
                    return Err(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let rt = self.gpr[instr.rt as usize];
                let rs = self.gpr[instr.rs as usize];
                let offset = (instr.imm as i16) as u64;
                let vaddr = rs.wrapping_add(offset);
                let vaddr_aligned = vaddr & !7;
                let value = self.read64(bus, vaddr_aligned)?;
                let byte_offset = vaddr & 7;
                let shift = 8 * (7 - byte_offset as u32);
                let mask = u64::MAX.checked_shl(64 - shift).unwrap_or(0);
                let result = (rt & mask) | (value >> shift);
                self.gpr[instr.rt as usize] = result;
            }
            32 => {
                // LB
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let value = self.read8(bus, vaddr)?;

                self.gpr[instr.rt as usize] = (value as i8) as u64;
            }
            33 => {
                // LH
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let value = self.read16(bus, vaddr)?;

                self.gpr[instr.rt as usize] = (value as i16) as u64;
            }
            34 => {
                // LWL
                let instr = ITypeInstruction::from_raw(i);
                let rt = self.gpr[instr.rt as usize] as u32;
                let rs = self.gpr[instr.rs as usize];
                let offset = (instr.imm as i16) as u64;
                let vaddr = rs.wrapping_add(offset);
                let vaddr_aligned = vaddr & !3;
                let value = self.read32(bus, vaddr_aligned)?;
                let byte_offset = vaddr & 3;
                let shift = 8 * byte_offset as u32;
                let mask = u32::MAX.checked_shr(32 - shift).unwrap_or(0);
                let result = (value << shift) | (rt & mask);
                self.gpr[instr.rt as usize] = (result as i32) as u64;
            }
            35 => {
                // LW
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let value = self.read32(bus, vaddr)?;

                self.gpr[instr.rt as usize] = (value as i32) as u64;
            }
            36 => {
                // LBU
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let value = self.read8(bus, vaddr)?;

                self.gpr[instr.rt as usize] = value as u64;
            }
            37 => {
                // LHU
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let value = self.read16(bus, vaddr)?;

                self.gpr[instr.rt as usize] = value as u64;
            }
            38 => {
                // LWR
                let instr = ITypeInstruction::from_raw(i);
                let rt = self.gpr[instr.rt as usize] as u32;
                let rs = self.gpr[instr.rs as usize];
                let offset = (instr.imm as i16) as u64;
                let vaddr = rs.wrapping_add(offset);
                let vaddr_aligned = vaddr & !3;
                let value = self.read32(bus, vaddr_aligned)?;
                let byte_offset = vaddr & 3;
                let shift = 8 * (3 - byte_offset as u32);
                let mask = u32::MAX.checked_shl(32 - shift).unwrap_or(0);
                let combined = (rt & mask) | (value >> shift);

                // Sign extend only if loading a full word.
                let result = if byte_offset == 3 {
                    combined as i32 as u64
                } else {
                    (self.gpr[instr.rt as usize] & 0xFFFFFFFF_00000000) | combined as u64
                };

                self.gpr[instr.rt as usize] = result;
            }
            39 => {
                // LWU
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let value = self.read32(bus, vaddr)?;

                self.gpr[instr.rt as usize] = value as u64;
            }
            40 => {
                // SB
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let data = self.gpr[instr.rt as usize] as u8;
                self.write8(bus, vaddr, data)?;
            }
            41 => {
                // SH
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let data = self.gpr[instr.rt as usize] as u16;
                self.write16(bus, vaddr, data)?;
            }
            42 => {
                // SWL
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let value = self.gpr[instr.rt as usize] as u32;
                let byte_offset = vaddr & 3;

                match byte_offset {
                    0 => {
                        self.write32(bus, vaddr, value)?;
                    }
                    1 => {
                        self.write8(bus, vaddr, (value >> 24) as u8)?;
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
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let data = self.gpr[instr.rt as usize] as u32;
                self.write32(bus, vaddr, data)?;
            }
            44 => {
                // SDL
                if self.reg_size == RegSize::Reg32 {
                    return Err(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let value = self.gpr[instr.rt as usize];
                let byte_offset = vaddr & 7;

                match byte_offset {
                    0 => {
                        self.write64(bus, vaddr, value)?;
                    }
                    1 => {
                        self.write8(bus, vaddr, (value >> 56) as u8)?;
                        self.write16(bus, vaddr + 1, (value >> 40) as u16)?;
                        self.write32(bus, vaddr + 3, (value >> 8) as u32)?;
                    }
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
                    return Err(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let value = self.gpr[instr.rt as usize];
                let vaddr_aligned = vaddr & !7;
                let byte_offset = vaddr & 7;

                match byte_offset {
                    0 => {
                        self.write8(bus, vaddr_aligned, value as u8)?;
                    }
                    1 => {
                        self.write16(bus, vaddr_aligned, value as u16)?;
                    }
                    2 => {
                        self.write16(bus, vaddr_aligned, (value >> 8) as u16)?;
                        self.write8(bus, vaddr_aligned + 2, value as u8)?;
                    }
                    3 => {
                        self.write32(bus, vaddr_aligned, value as u32)?;
                    }
                    4 => {
                        self.write32(bus, vaddr_aligned, (value >> 8) as u32)?;
                        self.write8(bus, vaddr_aligned + 4, value as u8)?;
                    }
                    5 => {
                        self.write32(bus, vaddr_aligned, (value >> 16) as u32)?;
                        self.write16(bus, vaddr_aligned + 4, value as u16)?;
                    }
                    6 => {
                        self.write32(bus, vaddr_aligned, (value >> 24) as u32)?;
                        self.write16(bus, vaddr_aligned + 4, (value >> 8) as u16)?;
                        self.write8(bus, vaddr_aligned + 6, value as u8)?;
                    }
                    7 => {
                        self.write64(bus, vaddr_aligned, value)?;
                    }
                    _ => unreachable!(),
                }
            }
            46 => {
                // SWR
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let vaddr_aligned = vaddr & !3;
                let value = self.gpr[instr.rt as usize] as u32;
                let byte_offset = vaddr & 3;

                match byte_offset {
                    0 => {
                        self.write8(bus, vaddr_aligned, value as u8)?;
                    }
                    1 => {
                        self.write16(bus, vaddr_aligned, value as u16)?;
                    }
                    2 => {
                        self.write16(bus, vaddr_aligned, (value >> 8) as u16)?;
                        self.write8(bus, vaddr_aligned + 2, value as u8)?;
                    }
                    3 => {
                        self.write32(bus, vaddr_aligned, value)?;
                    }
                    _ => unreachable!(),
                }
            }
            47 => { /* Not simulating CACHE for now, NOP */ } // CACHE
            48 => {
                // LL
                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let paddr = self.translate_vaddr(vaddr as u32)?;
                let value = self.pread32(bus, paddr)?;
                self.gpr[instr.rt as usize] = (value as i32) as u64;
                self.cp0.set_lladdr(paddr);
                self.llbit = true;
            }
            49 => {
                todo!("Instruction LWC1")
            } // LWC1
            50 => {
                todo!("Instruction LWC2")
            } // LWC2
            52 => {
                // LLD
                if self.reg_size == RegSize::Reg32 {
                    return Err(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let paddr = self.translate_vaddr(vaddr as u32)?;
                let value = self.pread64(bus, paddr)?;
                self.gpr[instr.rt as usize] = value;
                self.cp0.set_lladdr(paddr);
                self.llbit = true;
            }
            53 => {
                todo!("Instruction LDC1")
            } // LDC1
            54 => {
                todo!("Instruction LDC2")
            } // LDC2
            55 => {
                // LD
                if self.reg_size == RegSize::Reg32 {
                    return Err(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let value = self.read64(bus, vaddr)?;

                self.gpr[instr.rt as usize] = value;
            }
            56 => {
                // SC
                let instr = ITypeInstruction::from_raw(i);
                let value = self.gpr[instr.rt as usize] as u32;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add((instr.imm as i16) as u64);
                self.gpr[instr.rt as usize] = 0;

                if self.llbit {
                    self.write32(bus, vaddr, value)?;
                }

                self.gpr[instr.rt as usize] = 1;
            }
            57 => {
                todo!("Instruction SWC1")
            } // SWC1
            58 => {
                todo!("Instruction SWC2")
            } // SWC2
            60 => {
                // SCD
                if self.reg_size == RegSize::Reg32 {
                    return Err(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let value = self.gpr[instr.rt as usize];
                let vaddr = self.gpr[instr.rs as usize].wrapping_add((instr.imm as i16) as u64);
                self.gpr[instr.rt as usize] = 0;

                if self.llbit {
                    self.write64(bus, vaddr, value)?;
                }

                self.gpr[instr.rt as usize] = 1;
            }
            61 => {
                todo!("Instruction SDC1")
            } // SDC1
            62 => {
                todo!("Instruction SDC2")
            } // SDC2
            63 => {
                // SD
                if self.reg_size == RegSize::Reg32 {
                    return Err(CpuException::ReservedInstruction);
                }

                let instr = ITypeInstruction::from_raw(i);
                let offset = (instr.imm as i16) as u64;
                let vaddr = self.gpr[instr.rs as usize].wrapping_add(offset);
                let value = self.gpr[instr.rt as usize];
                self.write64(bus, vaddr, value)?;
            }
            _ => panic!("Unrecogized opcode: {opcode}"),
        }

        self.gpr[Self::ZR] = 0; // TODO: Move this to execute_instruction caller

        self.pc += 4;

        match self.exec_state {
            ExecutionState::Normal => {}
            ExecutionState::Jump { addr, taken } => {
                self.exec_state = ExecutionState::Delay { addr, taken };
            }
            ExecutionState::Delay { addr, taken } => {
                if taken {
                    // Execute a previously set-up branch/jump instruction.
                    self.pc = addr;
                }

                self.exec_state = ExecutionState::Normal;
            }
        }

        Ok(())
    }

    /// General branch instruction handler
    #[inline(always)]
    fn branch_instr<F>(&mut self, i: u32, cond: F, link: bool)
    where
        F: Fn(i64, i64) -> bool,
    {
        if rose_likely(!self.in_branch_delay_slot()) {
            let instr = ITypeInstruction::from_raw(i);
            let rs = self.gpr[instr.rs as usize] as i64;
            let rt = self.gpr[instr.rt as usize] as i64;
            let offset = ((instr.imm as i16) as u64) << 2;
            let branch_addr = self.pc.wrapping_add(offset).wrapping_add(4);

            if link {
                // Set link register to predicted instruction addr
                self.gpr[Self::LR] = self.pc.wrapping_add(8);
            }

            self.exec_state = ExecutionState::Jump {
                addr: branch_addr,
                taken: cond(rs, rt),
            };
        }
    }

    /// General branch likely instruction handler
    #[inline(always)]
    fn branch_likely_instr<F>(&mut self, i: u32, cond: F, link: bool)
    where
        F: Fn(i64, i64) -> bool,
    {
        if rose_likely(!self.in_branch_delay_slot()) {
            let instr = ITypeInstruction::from_raw(i);
            let rs = self.gpr[instr.rs as usize] as i64;
            let rt = self.gpr[instr.rt as usize] as i64;
            let offset = ((instr.imm as i16) as u64) << 2;
            let branch_addr = self.pc.wrapping_add(offset).wrapping_add(4);

            if link {
                self.gpr[Self::LR] = self.pc.wrapping_add(8);
            }

            if cond(rs, rt) {
                self.exec_state = ExecutionState::Jump {
                    addr: branch_addr,
                    taken: true,
                };
            } else {
                // TODO: Exception during this delay slot?
                self.pc = self.pc.wrapping_add(4); // Skip delay slot instruction
            }
        }
    }

    #[inline]
    pub const fn translate_vaddr(&mut self, vaddr: u32) -> Result<u32, CpuException> {
        match vaddr {
            0x00000000..=0x7FFFFFFF => Ok(0), /* KUSEG */ // TODO: User Segment, TLB mapped
            0x80000000..=0x9FFFFFFF => Ok(vaddr - 0x80000000), /* KSEG0 */
            0xA0000000..=0xBFFFFFFF => Ok(vaddr - 0xA0000000), /* KSEG1 */
            0xC0000000..=0xDFFFFFFF => Ok(0), /* KSSEG */ // TODO: Kernel Supervisor segment, TLB mapped
            0xE0000000..=0xFFFFFFFF => Ok(0), /* KSEG3 */ // TODO: Kernel Segment 3, TLB mapped
        }
    }

    fn read8(&mut self, bus: &mut Bus, vaddr: u64) -> Result<u8, CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            return Ok(unsafe { ptr.read_unaligned() });
        }

        let paddr = self.translate_vaddr(vaddr)?;

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

        let paddr = self.translate_vaddr(vaddr)?;

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

        let paddr = self.translate_vaddr(vaddr)?;

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

        let paddr = self.translate_vaddr(vaddr)?;

        let hi = bus.read32(paddr) as u64;
        let lo = bus.read32(paddr + 4) as u64;

        Ok(hi << 32 | lo)
    }

    /// Read a 64-bit value from the given physical address
    fn pread32(&mut self, bus: &mut Bus, paddr: u32) -> Result<u64, CpuException> {
        // Alignment check
        if rose_unlikely(paddr & 7 != 0) {
            return Err(CpuException::AddressErrorLoad);
        }

        Ok(bus.read32(paddr) as u64)
    }

    /// Read a 64-bit value from the given physical address
    fn pread64(&mut self, bus: &mut Bus, paddr: u32) -> Result<u64, CpuException> {
        // Alignment check
        if rose_unlikely(paddr & 7 != 0) {
            return Err(CpuException::AddressErrorLoad);
        }

        let hi = bus.read32(paddr) as u64;
        let lo = bus.read32(paddr + 4) as u64;

        Ok(hi << 32 | lo)
    }

    fn write8(&mut self, bus: &mut Bus, vaddr: u64, value: u8) -> Result<(), CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            unsafe {
                ptr.write_unaligned(value);
            }
            return Ok(());
        }

        let paddr = self.translate_vaddr(vaddr)?;

        bus.write8(paddr, value);

        Ok(())
    }

    fn write16(&mut self, bus: &mut Bus, vaddr: u64, value: u16) -> Result<(), CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;
        let value = value.to_be();

        // Alignment check
        if rose_unlikely(vaddr & 1 != 0) {
            return Err(CpuException::AddressErrorStore);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            unsafe {
                (ptr as *mut u16).write_unaligned(value);
            }
            return Ok(());
        }

        let paddr = self.translate_vaddr(vaddr)?;

        bus.write16(paddr, value);

        Ok(())
    }

    fn write32(&mut self, bus: &mut Bus, vaddr: u64, value: u32) -> Result<(), CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;
        let value = value.to_be();

        // Alignment check
        if rose_unlikely(vaddr & 3 != 0) {
            return Err(CpuException::AddressErrorStore);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            unsafe {
                (ptr as *mut u32).write_unaligned(value);
            }
            return Ok(());
        }

        let paddr = self.translate_vaddr(vaddr)?;

        bus.write32(paddr, value);

        Ok(())
    }

    fn write64(&mut self, bus: &mut Bus, vaddr: u64, value: u64) -> Result<(), CpuException> {
        // Expect ROMS to use 32-bit addressing always
        assert_eq!((vaddr as i32) as u64, vaddr);

        let vaddr = vaddr as u32;
        let value = value.to_be();

        // Alignment check
        if rose_unlikely(vaddr & 7 != 0) {
            return Err(CpuException::AddressErrorStore);
        }

        if let Some(ptr) = bus.memory.get_raw_mem(vaddr) {
            unsafe {
                (ptr as *mut u64).write_unaligned(value);
            }
            return Ok(());
        }

        let paddr = self.translate_vaddr(vaddr)?;

        bus.write32(paddr, (value >> 32) as u32);
        bus.write32(paddr, value as u32);

        Ok(())
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

        assert_eq!(cpu.read64(&mut bus, RDRAM_BASE), Ok(0x18192021_78798081));

        assert_eq!(cpu.write8(&mut bus, RDRAM_BASE, 0x00), Ok(()));
        assert_eq!(cpu.write8(&mut bus, RDRAM_BASE + 1, 0x00), Ok(()));
        assert_eq!(cpu.write8(&mut bus, RDRAM_BASE + 2, 0x00), Ok(()));
        assert_eq!(cpu.write8(&mut bus, RDRAM_BASE + 3, 0x00), Ok(()));

        assert_eq!(cpu.write16(&mut bus, RDRAM_BASE, 0x00), Ok(()));
        assert_eq!(cpu.write16(&mut bus, RDRAM_BASE + 2, 0x00), Ok(()));

        assert_eq!(cpu.write32(&mut bus, RDRAM_BASE, 0x00), Ok(()));

        assert_eq!(cpu.write64(&mut bus, RDRAM_BASE, 0x00), Ok(()));

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
            Err(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write16(&mut bus, RDRAM_BASE + 3, 0x01),
            Err(CpuException::AddressErrorStore)
        );

        assert_eq!(
            cpu.write32(&mut bus, RDRAM_BASE + 1, 0x01),
            Err(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write32(&mut bus, RDRAM_BASE + 2, 0x01),
            Err(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write32(&mut bus, RDRAM_BASE + 3, 0x01),
            Err(CpuException::AddressErrorStore)
        );

        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 1, 0x01),
            Err(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 2, 0x01),
            Err(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 3, 0x01),
            Err(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 4, 0x01),
            Err(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 5, 0x01),
            Err(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 6, 0x01),
            Err(CpuException::AddressErrorStore)
        );
        assert_eq!(
            cpu.write64(&mut bus, RDRAM_BASE + 7, 0x01),
            Err(CpuException::AddressErrorStore)
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
