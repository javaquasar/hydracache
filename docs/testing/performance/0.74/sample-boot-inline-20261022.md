# Owned inline boot cost comparison

## Decision

Do not adopt this representation. The pure Windows experiment removes the
short boot String allocation but makes the checked getter slower on both
compilers and increases point size from 40 to 56 bytes. This satisfies the
[preregistered rejection rule](sample-boot-inline-plan-20261022.md). Runtime
representation, all live checks and their original deadlines remain unchanged.

The prototype is retained only in the standalone
[test image](../../../../tools/sample-boot-inline-074/README.md), which has no
product or supervisor consumer. It is not a new full-path or server candidate.
Plan commit: `1e8552e8`; reference preflight: `4ec3b04d`; prototype and reader:
`fb036e3c785a00b497ea564d3775ac37e8fadd2d`.
The offline closing runner is `87b3e130`; retained evidence is `d8a3befc`.

## Hypothesis and semantic controls

The candidate copies at most 36 UTF-8 bytes into private inline storage, with
an owned String fallback for longer values. Safe from_utf8 checks every inline
getter; no unchecked conversion, borrowing, sharing or narrowed decoder is
used. Full point/getter and sample bodies are restored exactly by source
canaries after reversing only the representation substitutions. Both original
revalidations and both clock calls still precede construction; synthetic
receivers have identical implementations for both arms.

Both first installed MSVC compiler lanes, Rust 1.94 and MSRV 1.88, pass twelve
pure tests each. These include the old seven baseline/allocator controls plus
five candidate controls. All sixteen masks preserve first errors and event
prefixes, and all fifteen refusals precede construction. Nine corpus cases,
246 generic/boundary UTF-8/NUL/escape cases and 256 seeded UUIDs check values,
ownership after Origin mutation/drop, independent point clones, normal Debug
and alternate Debug. The original decoder remains unchanged.

## Allocation benefit and value size

The same nine cases run 256 sample calls per arm, per compiler. Both compiler
lanes agree on all allocation counters; each allocated sample drops inside its
capture. The unchanged baseline rows are reconstructed by the old reviewer.

| Per compiler and arm | Calls | Allocations | Requested bytes |
| --- | ---: | ---: | ---: |
| Reference main corpus | 2304 | 2048 | 324864 |
| Candidate main corpus | 2304 | 768 | 288000 |
| Reference fixed UUID and spare-capacity subset | 512 | 512 | 18432 |
| Candidate same subset | 512 | 0 | 0 |

The subset is already part of the main corpus. Lengths above 36 retain their
previous requests; no spare input capacity is copied into the fallback. Main
zeroed allocations and reallocations are zero, with matching deallocation
counts/layout bytes. Separately, 256 clones of a fixed 36-byte point change
from 256 allocations/9216 requested bytes to zero. Neither clone nor getter
controls are pooled into the main sample totals.

Both getter loops allocate nothing. The point grows from 40 bytes/alignment 8
to 56 bytes/alignment 8 on these Windows targets. This layout observation is
not a measurement of physical copy traffic, RSS or retained memory, and the
synthetic case weights are not production frequencies.

## Getter cost rejects adoption

Reference-only cost preflight was retained before the prototype existed. Its
seven-interval medians are 70400 ns on Rust 1.94 and 71100 ns on 1.88. These
standalone values are not subtracted from the paired observations.

Each paired phase warms both arms with 32768 calls, then retains all seven
interval pairs of 262144 calls each. Reference-first and candidate-first order
alternate. Both arms use the same 36-byte UUID, black_box reference and 9437184
length checksum. Allocator capture is inactive during timing; Instant is used
only in the explicitly bounded pure getter controls, never admission time.

| First paired phase | Reference median ns per interval | Candidate median ns per interval |
| --- | ---: | ---: |
| Windows MSVC Rust 1.94 | 60000 | 2105800 |
| Windows MSVC Rust 1.88 | 54800 | 2183900 |

The checked inline getter's repeated UTF-8 validation is a plausible source
of this extra leaf cost; the experiment does not separately attribute each
instruction. All first pairs are retained without retry or noise pruning.
These are local getter wall intervals, not native/RESP throughput, full-path
CPU/op or proof of a net preparation slowdown. They are sufficient for the
conservative rejection rule fixed before measurement. Removing the checked
getter to pass would be a different hypothesis, not a repair of this candidate.

## Evidence and remaining gates

The [retained packet](local-runs/sample-boot-inline-20261022/facts.json) includes
reference-only and candidate first logs/receipts, both source snapshots and a
read-only reconstruction. Fifteen older raw seals and 856 members are verified.
Its inventory seals 72 members with SHA-256
`6bee3c2c239d2e38f9ad06a9098e756edc8dc248b6c48eb51f425b61b36b4ebf`.
The first candidate host source gate failed because its restoration assertion
added one newline. The exact assertion repair and separate source-only rerun
are retained; no measurement or runtime body was changed or rerun. The reader's
initial development check also needed the exact rustfmt-shaped assertion repair
to be recognized; that was a reviewer fix, not a new observation.

Forty-six source canaries pass after the repair, alongside standalone checks
and strict clippy on both compilers, xtask check/clippy, thirty-six offline
reviewer/reconciliation controls, formatting and local structural gates. The
local performance contract remains non-promotable. This is a focused step gate,
not full release qualification.

No runtime rollback is needed: production never consumed this prototype.
Continue with an offline audit of remaining allocation leaves before choosing
another isolated candidate; do not turn this negative result into a new live
attempt. Original 300/60/10-second windows and first Causal(Time) ages
10042390247/10010835739 ns remain. Material/cleanup/delivery/all-writer, full
SIGKILL/lost-ACK, reconnect and server gates are still closed. No WSL, clock or
service change, installation, supervisor operation or server workload occurred.
