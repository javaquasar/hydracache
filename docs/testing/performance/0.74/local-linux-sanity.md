# Local Linux sanity tier

On 2026-10-02, source `58fefe04b8ab4e295edfe83a5cd2f1810c7b8f7c` was exercised in the
local Ubuntu WSL2 environment. `CARGO_TARGET_DIR=/tmp/hydracache-074-target` kept Linux objects
separate from the Windows workspace target. The focused native attribution suite passed 3/3 and
the adversarial RESP transport suite passed 4/4.

The machine-readable receipt is
[`local-runs/linux-sanity-20261002.json`](local-runs/linux-sanity-20261002.json).

This is a `local-quick` portability sub-tier, not an additional release tier. WSL2 is not a
dedicated host, its nightly toolchain does not match the frozen qualification toolchain, and
neither `perf` nor `strace` is installed. Consequently the result can catch build and semantic
differences but cannot close the kernel syscall gate, establish performance numbers, or promote a
candidate.
