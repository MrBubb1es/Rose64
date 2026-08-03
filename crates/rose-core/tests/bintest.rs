//!-----------------------------------------------------------------------------
//! VR4300/tests.rs: Instruction-level test cases for the VR4300 CPU.
//! 
//! Using Thar0's N64 cpu test binaries to test instruction behavior. These
//! are NOT full N64 roms, but raw binary files containing machine code, so the
//! CPU will be given some dummy RAM to work with and run in isolation.
//! 
//! Authors: logan (lpreston618), MrBubblezsz
//!-----------------------------------------------------------------------------


mod bintest {
    use rose_core::processors::vr4300::CpuVR4300;
    use rose_core::common::consts::KB;

    const TEST_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/cpu/");
    const TEST_MEM_SIZE: usize = 32 * KB;

    struct BinTestHeader {
        code_offset: u32,
        code_size: u32,
        initial_memory_offset: u32,
        final_memory_offset: u32,
        memory_chunk_size: u32,
        code_load_addr: u32,
        memory_load_addr: u32,
        extended_header_addr: u32,
    }

    struct BinTestFuncMeta {
        vram_addr: u32,
        size_bytes: u32,
    }

    struct BinTestExtendedHeader {
        header_type: u32,
        next_extended_header_offset: u32,
        num_funcs: u32,
        entrypoint_index: u32,
        funcs_meta: Vec<BinTestFuncMeta>,
    }

    struct BinTest {
        header: BinTestHeader,
        code_chunk: Vec<u8>,
        initial_memory: Vec<u8>,
        final_memory: Vec<u8>,
        // extended_headers: Vec<BinTestExtendedHeader>, // probably won't use these
    }

    struct TestMemory {
        mem: [u32; TEST_MEM_SIZE],
    }

    /// Read in four u8 and return a u32
    fn u32_at(data: &[u8], idx: usize) -> u32 {
        u32::from_be_bytes([
            data[idx+0], data[idx+1], data[idx+2], data[idx+3],
        ])
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
            extended_header_addr: u32_at(data, 28),
        };

        let code_start = header.code_offset as usize;
        let code_end = code_start + header.code_size as usize;
        let init_mem_start = header.initial_memory_offset as usize;
        let init_mem_end = init_mem_start + header.memory_chunk_size as usize;
        let final_mem_start = header.final_memory_offset as usize;
        let final_mem_end = final_mem_start + header.memory_chunk_size as usize;

        BinTest {
            header: header,
            code_chunk: data[code_start..code_end].to_vec(),
            initial_memory: data[init_mem_start..init_mem_end].to_vec(),
            final_memory: data[final_mem_start..final_mem_end].to_vec(),
        }
    }

    fn run_bin_test(test_file: &str) {
        const MAGIC_RETURN_ADDRESS: u32 = 0xDEAD0123;

        let test_path = format!("{TEST_DIR}{test_file}");
        let test_data = std::fs::read(test_path).unwrap();
        let bin_test = read_bin_test(&test_data);

        let cpu = CpuVR4300::new();
        
        // Todo:
        // - Load bin_test.code_chunk into memory starting at
        //    bin_test.header.code_offset.
        // - Set cpu.r31 (return address) to magic return address
        // - Cycle cpu until JR RA executed, then immediately halt
        // - Compare memory state to bin_test.final_memory
    }

    #[test]
    fn test_bin_addu_data() {
        run_bin_test("addu_data.bin");
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
}