# Build the exact frozen observer locally, without a workload

This new slice allows a local Linux release build of observer `62114be0`, not a
candidate, host install, signature enrollment or product workload. Earlier
verification-only contracts/packets remain immutable and still describe their
own narrower slice. The reviewed `BUILD_COMMAND` in `diagnostic_artifacts.rs`
is executed unchanged with Rust 1.94.0, empty features and its ordinary target
path. No allocation instrumentation, RUSTFLAGS or profile overrides are used.

The dedicated detached build worktree must have the exact commit/tree and clean
status before/after compilation. Checkout bytes of locks/configs must match
their frozen hashes; Windows CRLF checkout conversion is not acceptable for
include_bytes build identity. The first newly created, clean build checkout had
converted lock bytes and was replaced with a LF checkout before any compilation.
No existing product or qualification worktree was changed.

Retain native Cargo JSON stdout separately from compiler stderr. Check its one
unprofiled timing artifact and final success, bounded ELF structure, locks and
four P0 configs. Run the compiled binary only with `--validate CONFIG` on each
surface; require valid=true, fixture_started=false, admission_allowed=false.
No --run invocation, fixture, latency/CPU/allocation claim or trusted build
receipt is produced. Source identity is independently checked by the local
coordinator, not authenticated by the binary's validation output.

This reduces build-readiness uncertainty. A local developer build and raw Cargo
log are not trusted builder attestation or independently reproduced compilation.
Signed enrollment, live backend, policy/process/tree/artifact binding, writer
revocation, watchdog and uncertain-intent reconciliation still precede launch.
