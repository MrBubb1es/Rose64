//!-----------------------------------------------------------------------------
//! VR4300/tests.rs: Instruction-level test cases for the VR4300 CPU.
//!
//! Using Thar0's N64 cpu test binaries to test instruction behavior. These
//! are NOT full N64 roms, but raw binary files containing machine code, so the
//! CPU will be given some dummy RAM to work with and run in isolation.
//!
//! Authors: logocrazymon, MrBubblezsz
//!-----------------------------------------------------------------------------

#![cfg(test)]

use crate::common::consts::MB;
use crate::memory::bus::{Bus, MemoryAccess};
use crate::processors::vr4300::{CpuVR4300, RegSize};
use crate::processors::vr4300::disassembler::Disassembler;

const TEST_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-roms/vr4300-thar0/");
const TEST_ROM_SIZE: usize = 8 * MB;

/// Translate a KSEG0 or KSEG1 virtual address to physical address space.
fn translate_vaddr_simple(vaddr: u32) -> u32 {
    match vaddr {
        0x00000000..=0x7FFFFFFF => 0,
        0x80000000..=0x9FFFFFFF => vaddr - 0x80000000, /* KSEG0 */
        0xA0000000..=0xBFFFFFFF => vaddr - 0xA0000000, /* KSEG1 */
        0xC0000000..=0xDFFFFFFF => 0,
        0xE0000000..=0xFFFFFFFF => 0,
    }
}

struct BinTestHeader {
    code_offset: u32,
    code_size: u32,
    initial_memory_offset: u32,
    final_memory_offset: u32,
    memory_chunk_size: u32,
    code_load_addr: u32,
    memory_load_addr: u32,
    _extended_header_addr: u32,
}

// Unused for now
// struct BinTestFuncMeta {
//     vram_addr: u32,
//     size_bytes: u32,
// }

// Unused for now
// struct BinTestExtendedHeader {
//     header_type: u32,
//     next_extended_header_offset: u32,
//     num_funcs: u32,
//     entrypoint_index: u32,
//     funcs_meta: Vec<BinTestFuncMeta>,
// }

struct BinTest {
    header: BinTestHeader,
    code_chunk: Vec<u8>,
    initial_memory: Vec<u8>,
    final_memory: Vec<u8>,
    // extended_headers: Vec<BinTestExtendedHeader>, // probably won't use these
}

/// Read in four u8 and return a u32
fn u32_at(data: &[u8], idx: usize) -> u32 {
    u32::from_be_bytes([data[idx], data[idx + 1], data[idx + 2], data[idx + 3]])
}

/// Read in binary test header info from raw bytes.
///
/// # Arguments
///  - `data`: raw bytes of the .bin file
///
/// # Returns
///  - BinTest struct with header info and vectors containing the code
///    segment, initial memory state, and final mempory state.
fn read_bin_test(data: &[u8]) -> BinTest {
    let header = BinTestHeader {
        code_offset: u32_at(data, 0),
        code_size: u32_at(data, 4),
        initial_memory_offset: u32_at(data, 8),
        final_memory_offset: u32_at(data, 12),
        memory_chunk_size: u32_at(data, 16),
        code_load_addr: u32_at(data, 20),
        memory_load_addr: u32_at(data, 24),
        _extended_header_addr: u32_at(data, 28),
    };

    let code_start = header.code_offset as usize;
    let code_end = code_start + header.code_size as usize;
    let init_mem_start = header.initial_memory_offset as usize;
    let init_mem_end = init_mem_start + header.memory_chunk_size as usize;
    let final_mem_start = header.final_memory_offset as usize;
    let final_mem_end = final_mem_start + header.memory_chunk_size as usize;

    BinTest {
        header,
        code_chunk: data[code_start..code_end].to_vec(),
        initial_memory: data[init_mem_start..init_mem_end].to_vec(),
        final_memory: data[final_mem_start..final_mem_end].to_vec(),
    }
}

fn run_bin_test(test_file: &str) {
    const MAX_INSTRUCTIONS: usize = 100_000_000;
    const MAGIC_RETURN_ADDRESS: u64 = 0x00000000_DEAD0123;

    let test_path = format!("{TEST_DIR}{test_file}");
    let test_data = std::fs::read(test_path).unwrap();
    let bin_test = read_bin_test(&test_data);

    let blank_rom = vec![0u8; TEST_ROM_SIZE];
    let mut cpu = CpuVR4300::new();
    let mut bus = Bus::new(blank_rom).ok().unwrap();

    let code_start = bin_test.header.code_load_addr;
    let code_size = bin_test.header.code_size;

    let mem_start = bin_test.header.memory_load_addr;
    let mem_size = bin_test.header.memory_chunk_size;

    let pcode_start = code_start;
    let pcode_end = code_start + code_size;

    println!("Writing {code_size} bytes of code to MEM[{pcode_start:08X}..{pcode_end:08X}]");

    for i in 0..code_size {
        bus.write8(
            translate_vaddr_simple(code_start + i),
            bin_test.code_chunk[i as usize],
        );
    }

    for i in 0..mem_size {
        bus.write8(
            translate_vaddr_simple(mem_start + i),
            bin_test.initial_memory[i as usize],
        );
    }

    cpu.gpr[CpuVR4300::LR] = MAGIC_RETURN_ADDRESS;
    cpu.pc = (code_start as i32) as u64;
    cpu.reg_size = RegSize::Reg64;

    let is_jr_instr = |instr: u32| (instr >> 26) == 0 && (instr & 0x3F) == 0b001000;

    let mut instruction_count: usize = 0;
    for _ in 0..MAX_INSTRUCTIONS {
        let instr = bus.read32(translate_vaddr_simple(cpu.pc as u32));
        
        println!("0x{:08X}: {}", cpu.pc as u32, Disassembler::instruction_string(cpu.pc, instr));

        if is_jr_instr(instr) {
            let rs = (instr >> 21) & 0x1F;

            if cpu.gpr[rs as usize] == MAGIC_RETURN_ADDRESS {
                break;
            }
        }

        cpu.execute_instruction(&mut bus, instr).unwrap();
        instruction_count += 1;
    }

    println!("Finished '{test_file}' after {instruction_count} instructions.");

    let mut memory_result = Vec::new();

    for i in 0..mem_size {
        memory_result.push(bus.read8(translate_vaddr_simple(mem_start + i)));
    }

    for (i, (result, expected)) in memory_result
        .iter()
        .zip(bin_test.final_memory.iter())
        .enumerate()
    {
        assert_eq!(
            *result,
            *expected,
            "Mismatch at index {i}, address ${:08X}",
            mem_start + i as u32,
        );
    }
}

#[test]
fn test_bin_addu_data() {
    run_bin_test("addu_data.bin");
    // first Instr: 400B4800, Op: 010000
}

#[test]
fn test_bin_ddivu_data() {
    run_bin_test("ddivu_data.bin");
}

#[test]
fn test_bin_ddiv_data() {
    run_bin_test("ddiv_data.bin");
}

#[test]
fn test_bin_divu_data() {
    run_bin_test("divu_data.bin");
}

#[test]
fn test_bin_div_data() {
    run_bin_test("div_data.bin");
}

#[test]
fn test_bin_dmultu_data() {
    run_bin_test("dmultu_data.bin");
}

#[test]
fn test_bin_dmult_data() {
    run_bin_test("dmult_data.bin");
}

#[test]
fn test_bin_dsll32_data() {
    run_bin_test("dsll32_data.bin");
}

#[test]
fn test_bin_dsllv_data() {
    run_bin_test("dsllv_data.bin");
}

#[test]
fn test_bin_dsll_data() {
    run_bin_test("dsll_data.bin");
}

#[test]
fn test_bin_dsra32_data() {
    run_bin_test("dsra32_data.bin");
}

#[test]
fn test_bin_dsrav_data() {
    run_bin_test("dsrav_data.bin");
}

#[test]
fn test_bin_dsra_data() {
    run_bin_test("dsra_data.bin");
}

#[test]
fn test_bin_dsrl32_data() {
    run_bin_test("dsrl32_data.bin");
}

#[test]
fn test_bin_dsrlv_data() {
    run_bin_test("dsrlv_data.bin");
}

#[test]
fn test_bin_dsrl_data() {
    run_bin_test("dsrl_data.bin");
}

#[test]
fn test_bin_ldl_data() {
    run_bin_test("ldl_data.bin");
}

#[test]
fn test_bin_ldr_data() {
    run_bin_test("ldr_data.bin");
}

#[test]
fn test_bin_link_data() {
    run_bin_test("link_data.bin");
}

#[test]
fn test_bin_lui_data() {
    run_bin_test("lui_data.bin");
}

#[test]
fn test_bin_lwl_data() {
    run_bin_test("lwl_data.bin");
}

#[test]
fn test_bin_lwr_data() {
    run_bin_test("lwr_data.bin");
}

#[test]
fn test_bin_multu_data() {
    run_bin_test("multu_data.bin");
}

#[test]
fn test_bin_mult_data() {
    run_bin_test("mult_data.bin");
}

#[test]
fn test_bin_sllv_data() {
    run_bin_test("sllv_data.bin");
}

#[test]
fn test_bin_sll_data() {
    run_bin_test("sll_data.bin");
}

#[test]
fn test_bin_srav_data() {
    run_bin_test("srav_data.bin");
}

#[test]
fn test_bin_sra_data() {
    run_bin_test("sra_data.bin");
}

#[test]
fn test_bin_srlv_data() {
    run_bin_test("srlv_data.bin");
}

#[test]
fn test_bin_srl_data() {
    run_bin_test("srl_data.bin");
}
