# Progress

## CPU (VR4300)
- [ ] LLE:
  - [ ] Register file / ALU instructions
  - [ ] Load/store instructions
  - [ ] Branch/jump + delay slots
  - [ ] COP0 (system control)
  - [ ] COP1 (FPU)
  - [ ] Exceptions/interrupts
  - [ ] Passes community CPU instruction test ROMs
- [ ] HLE:
  - [ ] Set up fastmem access
  - [ ] Write cranelift IR for instructions
  - [ ] Implement basic block compilation
  - [ ] Superblock linking w/ RSP sync
  - [ ] Write custom dynarec for WASM target

## Bus / Memory
- [ ] Address decoding (RDRAM, cart, PIF, RSP mem)
- [ ] DMA (PI, SI)

## RSP
- [ ] Core stepping
- [ ] Vector unit
- [ ] HLE microcode execution
- [ ] LLE microcode execution

## RDP
- [ ] Command list parsing
- [ ] Software rasterizer produces correct static test pattern
- [ ] Software rasterizer runs commercial games
- [ ] Hardware accelerated rendering

## MMIO
- [ ] VI (video timing)
- [ ] AI (audio DMA)
- [ ] SI (controller input)
- [ ] PI (cart/PIF DMA)

## Boot
- [ ] HLE PIF boot reaches IPL3
- [ ] First homebrew ROM runs
- [ ] Commercial games booting
- [ ] LLE PIF implementation (look at other emulators)

## Tooling
- [x] Workspace scaffold
- [x] CI (fmt, clippy, build, test)
- [ ] rose-cli ROM runner + golden-image regression harness
- [ ] Disassembler covers full instruction set
