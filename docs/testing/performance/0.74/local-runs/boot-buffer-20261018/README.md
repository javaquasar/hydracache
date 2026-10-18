# Fixed boot ID buffer local candidate

The temporary boot-document reader uses a 38-byte stack buffer instead of the
65,537-byte Vec at baseline `3b8e73fe`. This is an isolated experimental runtime
candidate, not an accepted preparation source. Its [preregistered plan](../../boot-buffer-candidate-plan-20261018.md)
requires a separate plain same-host original-worker comparison and the
unchanged full preparation healthy/fault gate before runtime acceptance.

## Change and semantic controls

`diagnostic_kernel_origin.rs` still opens and checks the same procfs directories
and file, reads at offset zero once, closes the document before decoding, and
enforces the same canonical lowercase UUID and newline. It does not retry
short reads or EINTR. `bounded` remains unchanged for sender status and pidfd
documents. All original origin/namespace/clock observations, admission origins,
46 effect brackets and 300/60/10-second deadlines remain unchanged.

Seven boot tests compare the actual reader and decoder with the old algorithm:
empty/short/overlength documents, 9,472 single-byte mutations, 1,024 deterministic
documents at seed `0x0740b038`, all prefix lengths of a canonical document with
a trailing byte, five I/O errors, document trace counts and real proc reads.
Neither algorithm proves EOF if a read returns just the canonical prefix.
Oversized malformed input now refuses as `boot-document` rather than the old
test-only `proc-document-budget` site when the small buffer cannot observe its
total length. Public invalid/I/O results remain the comparison boundary.

The exact source adapter permits only the declared boot substitution and its
test registration. Restoring that substitution must recover all 353 old Rust
sources exactly. A missing or changed reader/predicate is refused, not hidden
by a general textual normalization. The prior kernel tracer canary now also
accepts an earlier refusal from this stricter adapter as detection of drift.

## Narrow allocation measurement

A separate integration-test image includes the exact temporary-reader source
and its own TLS counting allocator. That allocator is absent from supervisor
and libtest images. Each compiler runs five counterbalanced pairs of 32 reads,
with returned storage destroyed inside the counted scope. The baseline makes
32 allocations and requests 2,097,184 bytes per scope; the candidate makes zero
temporary-reader allocations. These are measured allocator requests, not copied
bytes, resident memory, CPU/op or whole-worker allocations. Returned boot
Strings and directory/path allocations remain outside this measurement.

The old-buffer control fails both the width and allocation assertions before
the replacement. The first harness build also found a test closure return-type
error; its log is retained separately from the subsequent behavioral red
control and green candidate. Expected caught panics test counter restoration
and refusal of nested capture; they are not failed candidate cases.

## Reproduction and remaining gate

Candidate source is `dc00ebf9139c61d26750fe7875952483330c0453`.
Windows and GNU 1.94.0/MSRV 1.88.0 affected all-target checks and strict
all-feature Clippy pass. Each GNU image passes the seven new boot tests and
the focused kernel/origin/worker, manager/exchange and trace suites. Windows
passes 44 portable tests; all three allocation images pass four controls.
All 249 source contracts, formatting, the 184-gate registry, docs and the local
non-promotable performance contract pass. Manual host-only cases remain
ignored, and the full milestone `cargo xtask verify` is not claimed.

From the 0.74 worktree, invoke this directory's `check.ps1 -Lane windows`, `-Lane 194`, `-Lane 188`
and then `-Lane final`. Each lane preserves its first receipt under
`target/boot-buffer-20261018/checks` and refuses to overwrite it. GNU lanes use
the existing local WSL caches; no server operation is performed. Evidence
capture and verification bind the selected source and retained receipts.
`capture.ps1 -SourceCommit <full-sha>` records fresh-after-execution allocation
image metadata and hashes; `verify.ps1 -SelfTest` checks the closed inventory,
seven source capsules against Git, fifteen allocation scopes and nine negative
parser controls. This selected source slice is not a complete compiler input
or toolchain attestation. `negative-old-reader.source` retains the pre-replacement
helper, not a complete snapshot of the initial failing harness.

No timing result or full-preparation improvement is claimed. The approved
nonexecuting preparation source remains `2ed04dc8`. Real authenticated start,
all-writer cessation, durable completion/later cells, their new SIGKILL/lost-ack
matrix, reconnect and production enrollment remain open. No installed artifact,
service, server workload or frozen qualification input changes here.
