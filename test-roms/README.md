# test-roms/

This directory holds (or documents how to fetch) freely-distributable
homebrew and test ROMs used for automated correctness testing —
**never commit copyrighted commercial ROMs to this repo.**

Good sources for free/homebrew N64 test content include community CPU
instruction test suites and homebrew demos designed specifically for
emulator validation. Actual ROM binaries should either be:

- Small enough and permissively-licensed enough to commit directly, or
- Fetched by a script (`fetch-test-roms.sh`, not yet written) that
  downloads them from their original source at CI/setup time, so the
  binaries themselves aren't tracked in git.

Once `rose-cli` supports it, each test ROM here should have a
corresponding "known good" result checked into `tests/` (e.g. a
framebuffer hash after N frames) so CI can catch regressions
automatically.
