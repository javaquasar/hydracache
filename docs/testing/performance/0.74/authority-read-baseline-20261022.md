# Fresh authority bounded read baseline

## Result and scope

Both first installed Windows MSVC lanes, Rust 1.94 and MSRV 1.88, agree on the
fixed synthetic corpus: 2304 healthy reads make 2048 allocations and request
1222656 bytes. Zeroed allocations and reallocations are zero. Separate stages
attribute requests to Vec construction; generic bounded reading adds none for
these nine cases. This is a reference baseline, not an adopted storage change.

The [preregistered protocol](authority-read-baseline-plan-20261022.md) starts at
`94114c62` and is recorded in commit `b7bcaebf`. The dependency-free
[test image](../../../../tools/authority-read-baseline-074/README.md) executes
generic std Read::take/read_to_end with synthetic File/Metadata/open receivers.
File is an owned token, not a real descriptor. These request counts do not
measure Linux File/syscalls, full-path CPU, RSS, physical copying, retention or
headroom inside the original admission window.
Probe/reviewer source: `ffe37214a3ed1c3cdc767d4e5ea9d722e14b3603`.
Evidence commit: `0455c291`; the inventory seals 45 members with SHA-256
`ed0d1199cbd8e105b3b1143173d67911719cf140d797f7c1dd192b20bf76ccc1`.

## Full copied bodies and fixed healthy corpus

Source canaries bind the complete Leaf declaration/open/read/revalidate,
Stamp declaration/of and regular predicate to their unchanged production bodies.
Synthetic receivers cannot open files, query clocks, launch processes or perform
network IO. The allocator and its controls are reused unchanged. Production
opener flags, caller latching and the surrounding complete runtime remain frozen.

Nine cases use seed 0x074a17b3 and 256 complete reads each. They cover empty,
one-byte, 65-byte marker, one-byte-fragmented marker, 256-byte record, 2048-byte
record, seven-byte-fragmented 2048-byte record, eleven-byte-fragmented 256-byte
record with one Interrupted, and 37-byte binary data. Budgets stay 65/2048.
Synthetic bytes are not accepted authority JSON and equal case weights are not
production frequencies.

| Per compiler | Healthy calls | Allocations | Requested bytes |
| --- | ---: | ---: | ---: |
| Whole nine-case corpus | 2304 | 2048 | 1222656 |
| Empty subset | 256 | 0 | 0 |
| Each nonempty case | 256 | 256 | 256 times its byte length |

Returned capacity equals input length here, and main-loop deallocation counts
and supplied layout bytes match allocations/requests. This does not mean memory
was returned to the OS. Each case also has separate one-call constructor,
generic read, drop, whole-return and whole-drop controls. Constructor plus read
matches whole-return; the drops match; their sum multiplied by 256 matches that
case's main counts. These stage calls are not extra main operations.

Read traces preserve EOF and standard-library Interrupted handling. For example,
the full 65-byte marker uses two synthetic Read callbacks including EOF, while
one-byte fragments use 66. Their allocation counts agree. Callback counts are
not syscalls, and allocation agreement does not establish CPU equivalence.

## First errors and ownership remain separate controls

All 32 open/before/parent/read/after masks and all 64 revalidation masks including
held metadata retain exact first-error/event prefixes. They contain 31 and 63
refusals respectively. Early read/revalidate refusals are allocation-free in
28/60 masks; three late refusals per phase already own a 65-byte allocation.
Those failure counts and requests are not pooled into the healthy main corpus.

The model checks all eleven held-stamp and after-stamp fields, five regular-file
predicates and parent-device mismatch. Nine additional boundaries cover premature
EOF, growth within/beyond budget, before-metadata over-budget, partial errors,
Interrupted and zero-budget cases. An over-bound input is limited to 66 read
bytes for budget 65 and refuses; a terminal error on the second seven-byte
fragment retains seven consumed bytes and never reaches after-metadata.

Seeded controls exercise 256 owned returns after token drop/receiver reset,
independent Vec clones and fresh-content drift with unchanged asserted metadata.
Fake token drop keeps the original and fresh owners distinct. This is not a
kernel descriptor-close, real metadata race, sticky-registry, SIGKILL or lost-ACK
proof. Caller latching is preserved by unchanged source, not reenacted by the
synthetic File receiver.

## Evidence and next decision

Both first compiler selections pass nine pure tests, check and strict clippy.
Forty-nine source canaries, fifty-five offline reviewer/reconciliation controls,
formatting, xtask check/clippy and local structural contracts accompany the
[retained facts](local-runs/authority-read-baseline-20261022/facts.json).
Seventeen prior seals and 955 members remain intact. Compiler lanes are kept
separate, even though their first counters agree. Initial build-only E0583 was
a missing controls path attribute, fixed before any measured tests; neither
measured selection was retried or filtered.

The next question is a separately preregistered transient-storage prototype that
does not overwrite retained Leaf.raw, skip a fresh open, shorten EOF observation
or drop the fresh File before comparisons. This baseline alone cannot admit that
prototype into runtime; its copying/value-size/read behavior and full-path cost
still need their own gate.

Original 300/60/10 windows, first Causal(Time) ages 10042390247/10010835739 ns,
qualification digest and rejected inline-boot result remain unchanged. Runtime
is unchanged. Material/cleanup, authenticated delivery/all writers, full fault/
lost-ACK, reconnect and server admission remain closed to new claims. No WSL,
clock/service change, installation, supervisor operation or server/product
workload occurred.
