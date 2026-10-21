# Local context document buffer allocation baseline

The next [offline-audited leaf](reserved-leaf-audit-20261021.md) now has a pure
allocation baseline before implementation. The existing zeroed Vec makes one
Rust request of `length + 1` bytes per construction on both GNU 1.94 and MSRV
1.88, with no reallocation. Production document lengths 33/37 therefore request
34/38 bytes. No stack replacement or other runtime candidate is implemented.

The [method](context-document-buffer-baseline-plan-20261021.md) distinguishes
those two lengths from the existing length-four internal fixture and extra
shape controls. The [packet](local-runs/context-document-baseline-20261021/README.md)
keeps initial compiler refusals, corrected controls, source snapshots and the
complete local check results separately.

The baseline source is `71638d9db2817c1a1d24edddb467e2cbf7a4ec9e`.
Its packet contains 40 members plus inventory, with raw inventory SHA256
`8db3e4df5579ec7a471886505191825609b5c839732670bece6702ae101afcb6`.

## Allocation scope

Nine lengths, 0, 1, 4, 32, 33, 36, 37, 38 and 63, are constructed 256 times each.
Only Vec construction, borrowed black-box access and length/zero checks occur
inside the allocation capture. Reporting and model event lists stay outside it.
This uses the existing opt-in Rust allocator counter, not a new product allocator.

| Per compiler pure construction cohort | Allocations | Reallocations | Requested bytes |
| --- | ---: | ---: | ---: |
| All nine lengths, 2304 constructions | 2304 | 0 | 64768 |
| Production-length subset, 512 constructions | 512 | 0 | 18432 |

The zero-length control still constructs a one-byte buffer; it does not claim
that an empty document is admissible. The extra generic sizes are not a new
workload or production input policy. These counts do not include metadata,
kernel reading, malloc usable-size, CPU, RSS, retained memory or a full Probe.
No counter is multiplied by historical LocalContext calls to invent savings.

## Frozen body and error schedule

The production worker-context file is byte-identical to `8717ea4f`. A pure
test model retains the whole `Document::read_with` body, substituting synthetic
Result-valued observations for exactly the two metadata operands. Whole-body
token comparison restores those operands and checks every remaining operation;
the only formatting difference ignored is token whitespace/comments. Mutation
canaries reject altered buffer length, read propagation, count and newline tests.

Successful model cases keep metadata/read/metadata order and exactly `length + 1`
zeroed bytes in the callback. Deterministic seed `0x074cba51` covers 384 successful
shape/content cases over six sizes. Other controls cover short/zero/overrun and
impossible returned counts, malformed content/newline/expected lengths, EINTR,
EIO and EBADF without retry, and metadata refusal/I/O before or after reading.
Read errors skip closing metadata as in the original body; closing metadata
failure precedes later content checks. An initial metadata refusal also precedes
the allocation arithmetic even for a synthetic maximum usize length.

These are synthetic-body controls, not fresh kernel observations or real
metadata-race tests. They create no file, worker, cgroup, manager channel or
original owner. Real race coverage is a separate future gate, not inferred here.

## Source and local check results

The only edit to an old supervisor source file is a private module hook under
the existing opt-in test allocator. Exact reversal restores that complete file.
Every other old supervisor source/test file remains unchanged from `8717ea4f`.
In particular local document metadata/read/content checks, all full frames,
fresh check counts/order, absolute deadlines and refusal latches are unchanged.
No default product path, wire/durable artifact, native API or qualification input
changes. Historical guards keep whole-file comparisons, not body exclusions.

The first GNU compile check refuses the json-macro expression; MSRV also reports
an ambiguous numeric type. Their logs and source stay retained. Parenthesizing
the expression and annotating usize fixes only the new controls. The corrected
lane passes five new pure tests plus two existing allocator-counter controls per
compiler. Twenty-two Windows source canaries, affected all-target checks and
strict all-feature clippy pass, together with complete workspace formatting,
doc-check, the unchanged 186-gate registry and the local non-promotable contract.

Five preceding packets retain their raw seals and all 209 members. Original
windows remain 300/60/10 seconds and the first Causal(Time) ages remain
10042390247/10010835739 ns. The qualification manifest SHA256 is unchanged:
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
No ignored/manual test, live profile/retry, clock/service change, server,
installation, supervisor operation or product workload is run.

## Next isolated candidate

A short stack scratch remains a hypothesis. It must preserve exact callback
slice length, zero initialization, metadata/read/error ordering, the internal
fixture and generic compatibility rather than silently narrowing the length
argument. Larger shapes need the existing behavior, not truncation. Keep any
implementation and its source reversal separate from this frozen baseline.

No allocation reduction or timing improvement is claimed by a baseline alone.
It does not close full material admission, successful cleanup/EOF/fence,
authenticated delivery, all writers, SIGKILL/lost-ACK/reconnect or server gates.
