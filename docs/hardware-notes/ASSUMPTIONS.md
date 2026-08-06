- Assuming 32 and 64 bit versions of MULT/DIV instructions work identically WRT
  sign extension for now.
- Assuming most if not all ROMs only use 32-bit addressing, high 32-bits of
  CPU VAddresses are discarded.
- Two branch/jump instructions in a row is technically undefined behavior. We 
  assume that the first branch instruction executed will take effect (i.e. if we
  BRA 10, BRA 20, we will branch 10 ahead, not 20).
- Assuming we can execute the BranchLikely (BEQL, BGTZL, etc.) instructions
  without worrying about the delay slot - we simply branch on the spot and work
  out the timing.