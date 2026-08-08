//! -----------------------------------------------------------------------
//! instructions.rs: Some helper structs for different instruction types
//!
//! Author(s): logocrazymon
//! -----------------------------------------------------------------------

/// Data for an I-type instruction. An I-Type instruction has the structure:
///
/// <pre>
/// +-----------------+-------+-------+----------------+
/// | opcode (6 bits) | rs(5) | rt(5) | immediate (16) |
/// +-----------------+-------+-------+----------------+
/// </pre>
pub(crate) struct ITypeInstruction {
    // pub opcode: u8,
    pub rs: u8,
    pub rt: u8,
    pub imm: u16,
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
    pub fn from_raw(i: u32) -> ITypeInstruction {
        ITypeInstruction {
            // opcode: (i >> 26) as u8,
            rs: ((i >> 21) & 0x1F) as u8,
            rt: ((i >> 16) & 0x1F) as u8,
            imm: i as u16,
        }
    }
}

/// Data for a J-type opcode. A J-Type instruction has the structure:
/// <pre>
/// +-----------------+------------------------------------+
/// | opcode (6 bits) |          target (26 bits)          |
/// +-----------------+------------------------------------+
/// </pre>
pub(crate) struct JTypeInstruction {
    // pub opcode: u8,
    pub target: u32,
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
    pub fn from_raw(i: u32) -> JTypeInstruction {
        JTypeInstruction {
            // opcode: (i >> 26) as u8,
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
pub(crate) struct RTypeInstruction {
    // pub opcode: u8,
    pub rs: u8,
    pub rt: u8,
    pub rd: u8,
    pub shift: u8,
    // pub funct: u8,
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
    pub fn from_raw(i: u32) -> RTypeInstruction {
        RTypeInstruction {
            // opcode: (i >> 26) as u8,
            rs: ((i >> 21) & 0x1F) as u8,
            rt: ((i >> 16) & 0x1F) as u8,
            rd: ((i >> 11) & 0x1F) as u8,
            shift: ((i >> 6) & 0x1F) as u8,
            // funct: (i & 0x3F) as u8,
        }
    }
}