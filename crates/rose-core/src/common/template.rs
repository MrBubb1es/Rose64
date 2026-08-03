//! -----------------------------------------------------------------------
//! Filename.rs: A short description of the file's purpose
//!
//! A slightly longer description of the file. Include a brief overview of
//! its contents, how it works, and how it will be used.
//!
//! Author(s): MrBubblezsz
//! -----------------------------------------------------------------------

use crate::common::types::*;

const DESCRIPTIVE_NAME: U16 = n!(0x148E);

/// A brief description of the struct
/// 
/// # Section
/// Details about what the struct does, why it exists, how it is used, etc.
struct MyStruct {
    /// Description of public field
    pub field: U16,
}

impl MyStruct {
    pub fn new() -> MyStruct {
        MyStruct {
            field: todo!(),
        }
    }

    /// Adds `value` to `self.field`.
    ///
    /// # Arguments
    /// - `value`: The `u16` value to add.
    ///
    /// # Examples
    /// ```rust
    /// # use your\_crate::MyStruct;
    /// let mut s = MyStruct { field: 10 };
    /// s.add\_u16(5);
    /// assert\_eq!(s.field, 15);
    /// ```
    pub fn add_u16(&mut self, value: u16) {
        self.field += value.as_::<U16>();
    }
}

mod tests {
    use super::*;

    #[test]
    /// Verifies that `add_u16` correctly updates `field` using u16 input.
    fn test_mystruct_add() {
        let mut test_struct = MyStruct::new();

        assert_eq!(test_struct.field, 0.as_::<U16>());
        test_struct.add_u16(0x11EE);
        assert_eq!(test_struct.field, 0x11EE.as_::<U16>());
    }
}