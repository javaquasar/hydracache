# Expected inventory local control

This packet accepts only removal of expected-set materialization in the shared
authority fixture reader. The observed set and fresh scan remain. It does not
admit a server, cache workload, rebuilt tool or release.

Before source is `243288427e5fcc123e70319d233f939ab7291537`; After is
`5f56a9c3a1803ee7d403a411f091e98848a9e276`. Runtime substitution is
`7595dba2837627ac51fc66bb05a89f6b2a59521f`; source/template formatting only is
`c86330ae85a2ff9fc91f2d177ac2ae3f0e9fc48b`. The TSV lists every commit from
the preregistered baseline through the final capture source. No push occurs.

## Method and results

Frozen reference/proposal compare prepared observed sets outside setup cost.
Each compiler (GNU 1.94, MSRV 1.88) has one Before and one After invocation,
32 warmups, three alternated 1024-operation samples. Actual filesystem tests
check the runtime inventory; the unchanged eight-case full-inspect control
independently captures allocator, proc I/O and kernel observations. No failed
measurement is retried or replaced. All samples and both compilers agree.

| Operation | Allocations | Reallocations | Requested bytes |
| --- | ---: | ---: | ---: |
| Isolated equality, 1 generation, reference to stream | 4 to 2 | 4 to 4 | 440 to 112 |
| Isolated equality, 7 generations, reference to stream | 18 to 14 | 28 to 28 | 2056 to 784 |
| Isolated equality, 128 generations, reference to stream | 283 to 256 | 512 to 512 | 33912 to 14336 |
| Isolated empty equality | 0 to 0 | 0 to 0 | 0 to 0 |
| Isolated same-cardinality invalid 7-generation set | 18 to 2 | 28 to 4 | 2056 to 112 |
| Complete fixture inspect, 1 generation, Before to After | 33 to 29 | 8 to 8 | 2598 to 1942 |
| Complete fixture inspect, 7 generations, Before to After | 137 to 129 | 56 to 56 | 12612 to 10068 |

Each row is per operation, divided from raw 1024-operation totals. Complete
inspect saves exactly twice isolated healthy equality: 4 allocations/656 bytes
at depth one and 8/2544 at seven. Reads remain 4/28 per operation; six other
cases and every tracked kernel count remain equal. Invalid equality stops
generating pure expected strings early, not observing fewer directory entries.
The existing name formatter remains; healthy reallocations do not decrease.
No full-inspect claim is made at depth 128.

These are successful Rust allocator requests, including reallocation sizes in
requested bytes. They are not copied bytes, retained memory, CPU, latency,
complete syscall coverage, full ActiveAuthority or cache API measurements.
Before and After boots differ; no continuity or stable-clock claim follows.

## Checks, inputs and integrity

Both final compiler lanes pass 193 authority tests (13 manual/privileged tests
ignored), focused kernel/trust/counter checks, all-target check and strict
all-feature clippy. All 371 historical/current assertions pass both before and
after the whitespace-only repair. Final repository formatting, documentation,
186-gate registry and local non-promotable performance contract pass.

The first additional canary's extraction error is documented. Original MSRV
lint refusal and repository format refusal retain their logs/receipts. The
no-overwrite prefix refusal executes no check; the repository-only layout
recipe avoids collision without weakening the guard. All repairs precede the
first After measurement; no old assertion or runtime predicate is changed.

Effective recipes are `profile-before.ps1`, `profile-after-format.ps1`,
`repair-check.ps1`, `after-format-check.ps1`, `layout-repository-check.ps1`,
`canary-check.ps1`, `final-check.ps1` and `review-structural.ps1`. Original recipes stay
retained. Review reconstructs facts from raw observations, checks committed
inputs/images/recipes/check logs, exact depth conservation and whitespace-only
repair, rejects ten summary scope mutations, and rehashes eleven prior packets.
Seal once with `review-structural.ps1 -Mode Seal`; later use `-Mode Review` only.
The first string-based summary comparison refuses process-dependent Hashtable
key order before inventory publication. Its documented refusal and original
summary/verifier remain unchanged. Structural equality additionally passes
object-key permutation and refuses eight type/value/field/array mutations;
it cannot rewrite the summary and does not rerun any capture.

Changed runtime: `tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs`.
Test-only hooks/controls live in the allocator test module and
`tests/support/authority_inventory_{controls,alloc}.rs`; exact source guards
and two contract tests live under `crates/xtask/tests/authority_inventory_074.rs`
and its support files. The gate registry and private attribution contract are
updated. No public API, wire/durable format, policy or authority pin changes.

Workload/reference/qualification blobs remain identical. Qualification manifest
raw SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
Every live check and original 300/60/10 window remains. Old clock-order,
original-delivery, all-writer cessation, lifecycle, SIGKILL/lost-ACK, reconnect
and real enrollment gates stay independently closed. No new live matrix,
server run, installation, supervisor, clock/service or product operation occurs.
