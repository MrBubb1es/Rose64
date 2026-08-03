# Architecture & Design Decisions

This document records the "why" behind structural decisions so we
don't relitigate them mid-project.

## Open decisions (fill these in before/while building the relevant piece)

- **VR4300 execution model**: interpreter first (agreed). Implement JIT/dynarec
  after interpreter is thoroughly tested and verified against know working
  implementations.
- **RSP microcode**: Start with HLE implementation, then implement LLE 
  microcode for games that use custom or exact timing based microcode systems.
  - HLE: recognize known microcode (audio/graphics ucode) and
    reimplement its effect natively — faster, but breaks on
    unrecognized/modified microcode (some games use custom ucode).
  - LLE: actually execute RSP instructions — slower, but correct by
    construction for anything that boots.
- **RDP**: software rasterizer first (agreed, via the `Rasterizer`
  trait in `rose-core::rdp`). Hardware-accelerated backend is a later
  addition behind the same trait, not a rewrite.
- **PIF boot**: Start with HLE and investigate how other emulators handle LLE
  - HLE: emulate PIF results and skip straight to game entry point.
  - LLE: more accurate and handles CIC variants correctly, but is more work
  up front.
- **Timing accuracy**: Ideally with everything using LLE & lockstep execution,
  we will be able to emulate games with high levels of cycle-accuracy. For HLE
  execution we sacrifice most of that accuracy to get the benefit of higher
  performance. This will break some games. The default execution model will be
  HLE, but we will include a database of games known to work better or even
  require LLE execution.

## Crate boundaries

- `rose-core` - all emulation logic, zero I/O/windowing dependencies.
  Must build and run headless (this is what CI/test-roms exercise).
- `rose-app` - rendering (wgpu), audio (cpal), input. Talks to `rose-core`
  only through its public API.
- `rose-desktop` - cross platform windowing (winit), file dialogues (rfd),
  ROMs library detection, etc.
- `rose-web` - eventual WASM frontend
- `rose-cli` - headless ROM runner for test automation.

# CPU VR4300

We use a 2-tiered dynamic recompilation (dynarec) system:
- Tier 0: Interpreted, safest execution mode that serves as a fallback in case tier 1 fails or a block has not been compiled yet.
- Tier 1: Basic translation to native host instructions via cranelift.

We use the cranelift crate to handle recompilation of MIPS III instructions to native instructions. A JIT block accepts two parameters: A pointer to the CPU, and a base memory pointer.

## Milestones

See `docs/PROGRESS.md` for current status. Rough order:

1. CPU interpreter passes instruction-level test ROMs
2. HLE PIF boot sequence reaches IPL3
3. Simple homebrew ROM runs headless via `rose-cli`
4. HLE RDP produces a correct static test pattern
5. Commercial game reaches a title screen
6. Input, audio, RSP microcode online
7. Save states, compatibility pass
8. Basic block dynarec for VR4300, allow RCP to execute async.
9. Texture/model hot-swapping via texture/model data hashing