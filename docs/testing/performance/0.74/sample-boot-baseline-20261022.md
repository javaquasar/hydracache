# Sample boot ownership baseline

## Result and scope

In the Windows model, the valid 36-byte boot shape requires one 36-byte Rust
allocation when the current sample body clones its owned String. That clone
follows both original revalidations and both clock calls. All fifteen refused masks in the synthetic
four-call fault matrix return before the clone. Runtime and representation
remain unchanged; this is an allocation baseline, not a speedup or live gate.

The [preregistered baseline](sample-boot-baseline-plan-20261022.md) uses the
installed Windows MSVC compilers 1.94.0 and 1.88.0. Both first selections pass
seven pure tests, standalone check and strict clippy. This is not GNU/Linux
kernel evidence or a new attempt at the original material path.

Runtime source: `79dcf64bee22d801dc983ef62b4c2fefa44a0988`.
Probe/reviewer source: `f9eaf81d86a7a43f4d183b109b0e0d40cc7c363c`;
preregistration: `965b924d`.

## Complete frame and ownership contract

The [dependency-free test image](../../../../tools/sample-boot-baseline-074/README.md)
copies the complete sample body, KernelTimePoint declaration/getters/Clone
derive and boot decoder. Synthetic Origin, trace, error and clock receivers
provide no kernel or time capability. The symbolic clock selectors do not link
to libc. The allocator and its two controls are reused unchanged from the
previous proc document baseline, not added to production.

Three new source canaries compare all copied bodies, preserve the unchanged
runtime tree and bind the separate stage prelude to the complete original
prelude. Mutation controls reject removed/reordered calls, different clock
selectors, borrowed boot storage, changed getters, missing Clone and weakened
decoder bounds/alphabet. The surrounding production file is unchanged, including
actual clock conversion, namespace/boot observations and admission callers.

The healthy model order is span, revalidate, boot clock, real clock, revalidate,
then owned point construction. Sixteen masks cover the four fallible calls;
the first set bit selects the error and later calls do not execute. The fifteen
refused model paths make no allocator requests. This is not a claim that real
revalidations or kernel refusals allocate nothing, nor a real SIGKILL test.

## Matched first Windows allocation corpus

Nine cases run 256 samples each. They include empty and lengths 1, 35, 36, 37,
64 and 1024, a valid 36-byte UUID with spare input capacity, and a generic
36-byte UTF-8 value. Generic cases test String ownership compatibility, not
accepted Origin boot identities. The unchanged decoder still accepts only its
original bounded lowercase-hex UUID document shape.

Both compilers agree on the same corpus and subset:

| Per compiler | Sample calls | Allocations | Requested bytes | Reallocations |
| --- | ---: | ---: | ---: | ---: |
| Whole nine-case main corpus | 2304 | 2048 | 324864 | 0 |
| Fixed UUID and spare-capacity subset | 512 | 512 | 18432 | 0 |

The 256 empty-string calls make no allocation. Zeroed allocations are zero
throughout this corpus. Each allocated point drops inside its main-loop capture;
deallocation counts and supplied layout bytes equal the main allocation counts
and requested bytes. Input capacity 256 in the fixed-spare case produces a
36-byte request/output capacity, rather than copying spare capacity.

The fixed subset is already part of the main row, not extra operations. Separate
one-case stage controls for all nine cases compare prelude, clone, drop, whole
sample returning ownership, and drop of that returned point. The synthetic
prelude requests nothing; clone and returning sample have identical counters,
as do the two drops. Stage calls are not pooled into the main corpus.

These counts describe Rust requests on Windows, not physical copying, libc
malloc, CPU, RSS, retained memory or preparation headroom. Synthetic case
weights are not production frequencies. No old trace count is multiplied by
these differences, and no elapsed-time gain is claimed.

## Semantic and offline controls

Seed `0x074b0a19` covers 256 valid boot payloads, exact getter values including
u64 boundary arithmetic, owned points after Origin mutation/drop and independent
point clones. Point-clone property operations are not measured main samples.
The decoder control preserves newline, byte count, lowercase hex and hyphens,
including five invalid counts and seven invalid byte shapes. It does not test
real proc reads or clock acquisition.

The host passes 43 source canaries, xtask all-target check and strict all-feature
clippy. Direct formatting, eight new and twenty prior offline controls,
doc-check, unchanged 186-gate registration, protected-source comparison and the
local non-promotable performance contract pass. No full milestone gate is run.

The reviewer verifies fourteen preceding raw seals and all 813 members, the
preceding reviewer's sealed identity, seven probe-source pins, exact first
receipts/test names, compiler/platform, every case/stage/fault/seed/decoder row,
and strict numeric/boolean distinctions. No probe is invoked during review.
The inherited counter's caught unwind is an intentional passing self-test.

Original 300/60/10-second windows, Causal(Time) ages 10042390247/10010835739 ns,
qualification, root manifest/lock, product/runtime/observer sources and gate
registration remain. No WSL/live profile, clock/service change, installation,
supervisor operation, server action or product workload occurs. Material,
cleanup/EOF/fence, authenticated delivery/all writers, full SIGKILL/lost-ACK,
reconnect and server gates remain closed.

## Next isolated decision

An owned inline representation with generic String fallback is a possible
separate candidate, not an implementation or accepted benefit here. Before
adoption it needs exact ownership/error/frame equivalence, the matched baseline,
and review of Debug/getter compatibility, value size, copying and point-clone
costs.
Removing one allocation must not introduce borrowed/shared authority or shift
unmeasured work into callers. Keep current storage until that separate gate.

## Retained packet and replay

The [facts](local-runs/sample-boot-baseline-20261022/facts.json) retain both
first Windows lanes, all cases/stages/faults and preceding source/refusal pins.
The packet has 43 members plus its inventory, including the inherited counter,
method/current-source snapshots, first logs/receipts and focused checks.
Raw inventory SHA-256:
`fd07cd7911ab50d51e5a471de9ae5b427ec4076636396fe12861c46433ac6b10`.

```powershell
python -B tools/sample-boot-baseline-074/review_boot.py --output docs/testing/performance/0.74/local-runs/sample-boot-baseline-20261022/facts.json --verify --packet-sha256 fd07cd7911ab50d51e5a471de9ae5b427ec4076636396fe12861c46433ac6b10
```

Replay is source-pinned and read-only, without another probe. A future candidate
needs a separate identity; do not weaken this baseline to accept source drift.
