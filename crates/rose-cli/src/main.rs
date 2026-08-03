//! -----------------------------------------------------------------------
//! main.rs: Headless test-rom runner.
//!
//! CI calls this against the test-roms/ suite and compares output
//! hashes against checked-in "known good" values, so a regression in
//! CPU/RSP/RDP behavior fails the build instead of going unnoticed.
//!
//! Intended usage once fleshed out:
//!   rose-cli run <rom.z64> --frames 60 --hash-out result.txt
//!
//! Author(s): MrBubblezsz
//! -----------------------------------------------------------------------

fn main() {
    println!("rose-cli: stub, no ROM loading yet.");
}
