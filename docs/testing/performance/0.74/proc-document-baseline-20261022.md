# Proc document constructor and reuse baseline

## Result and decision

The remaining invocation-local DocumentBuffer performs one zeroed 65537-byte
Rust allocation at construction and reuses it without further allocator requests
for the four synthetic reads in this corpus. Keep the runtime unchanged. This
baseline does not establish a timing bottleneck or justify a new storage change.

The earlier [custody preparation experiment](local-runs/custody-preparation-cost-20261020/README.md)
already adopted per-invocation reuse at `1f371549`; its complete Sender comparison
is not repeated or presented as a new gain. The new work follows the
[preregistered pure baseline](proc-document-baseline-plan-20261022.md) and narrows
the remaining operation, rather than adding another whole-path cost claim.

Runtime source: `5310113d3a104b97f09325b4ac7f89c7f85263c4`.
Probe and reviewer source: `fa5df665dc20313d7461225b58a63d74f792b6ff`;
preregistration: `e519e88d`.

## Model and source boundary

The [separate test crate](../../../../tools/proc-document-baseline-074/README.md)
has no dependencies, production API or binary target. Its test-image allocator is
not linked into the supervisor. The private model keeps the complete current
DocumentBuffer declaration, constructor and read_with body, including the trace
scope, callback, budget and returned slice. A source canary omits only the exact
File/read_at adapter and compares the remaining complete token stream.

Synthetic trace/error receivers replace runtime observation receivers. These
tests do not exercise a real File, poll, procfs, namespace, credential, kernel
clock or Sender. Source canaries independently require unchanged runtime files
and the entire Sender source, including one buffer construction and the original
two pidfd/two status reads. No fresh observation, error site or window changes.

## First Windows results

Both installed Windows MSVC toolchains, Rust 1.94.0 and 1.88.0, pass the first
seven-test release-mode selection. Each executes 256 complete buffer instances
with four reads of lengths 16, 37, 4096 and 128, in that order. Those lengths are
a synthetic corpus, not a measured distribution of proc documents.

| Per compiler, 256 instances and 1024 callbacks | Observed requests |
| --- | ---: |
| Allocations | 256 |
| Of which alloc_zeroed | 256 |
| Reallocations | 0 |
| Requested bytes | 16777472 |
| Deallocations | 256 |
| Layout bytes supplied to deallocation | 16777472 |

A separate one-instance stage control measures construction, four reads and
drop independently and conserves the whole-instance counters. Construction
requests one zeroed allocation of 65537 bytes; reuse requests nothing; drop
supplies one 65537-byte layout to deallocation. These stage instances are not
pooled into the main 256-instance corpus.

Zeroed allocator requests do not measure physical zeroing, copies, usable heap
size, CPU, RSS, retained memory or live preparation time. Deallocation requests
do not prove that memory is returned to the OS. Windows model results are not
GNU/Linux runtime evidence. No elapsed-time or improvement claim is accepted.

## Error and ownership controls

The controls retain a single callback per read. Seed `0x074d5a18` drives 256
payload/reuse cases across empty, short, maximum and over-budget counts. Only
the fresh returned prefix is visible; long-to-short reads may retain an old
tail in scratch but never include it in the result. Independent buffers do not
alias, and a new buffer starts zeroed.

Three error kinds, including Interrupted, and three raw OS error payloads are
propagated without retry or a successful view. Raw codes on Windows test payload
preservation, not Linux errno interpretation. The nested/unwind/thread-isolation
and zeroed/realloc/drop controls verify the allocator itself. The caught panic
is an intentional counter-unwind test; the exact seven tests all finish green.

## Checks and evidence

Both Windows compilers pass standalone all-target check and strict all-feature
clippy. The host passes 40 source canaries, xtask all-target check and strict
all-feature clippy. Direct rustfmt, eight new offline reviewer controls, twelve
prior reconciliation controls, doc-check, unchanged 186-gate registration and
the local non-promotable performance contract pass. This is not full milestone
verification or a real-IO/fault gate.

The reviewer verifies thirteen preceding raw seals and all 770 members, seven
probe-source hashes, first receipts, compiler identity, exact test names and
rows, strict bool/integer distinctions, seed/error contracts and stage
conservation. It reconstructs the preceding reconciliation without weakening
any frozen reviewer. No probe is repeated during review.

Runtime, observer, product sources, root lock/manifest, compatibility,
qualification and gate registration remain unchanged. First Causal(Time) ages
10042390247/10010835739 ns and 300/60/10-second windows remain. No WSL/live
profile, clock/service change, server action, installation, supervisor operation
or product workload occurs. Material, cleanup/EOF/fence, authenticated delivery/
all writers, full SIGKILL/lost-ACK, reconnect and server gates stay closed.

## Next bounded work

Do not reduce the read slice/budget, remove zero initialization, cache observations
or move this 65537-byte storage onto nested stack frames on the strength of an
allocation count. A storage candidate needs its own compatible design and
attribution. The next separate pure baseline can isolate boot ownership in
Origin::sample while keeping both Origin revalidations and both clock samples
in the unchanged surrounding frame. No representation change is included here.

## Retained packet and replay

The [facts](local-runs/proc-document-baseline-20261022/facts.json) retain both
first Windows lanes, separate stages, probe source pins and all thirteen prior
inventory pins. The packet has 43 members plus its inventory, including the
first logs/receipts, method/current-source snapshots and final focused checks.
Raw inventory SHA-256:
`dbaa727197cf3c08b5206b40050bef4424f8a1e1793191ca661ba5540a81c306`.

```powershell
python -B tools/proc-document-baseline-074/review.py --output docs/testing/performance/0.74/local-runs/proc-document-baseline-20261022/facts.json --verify --packet-sha256 dbaa727197cf3c08b5206b40050bef4424f8a1e1793191ca661ba5540a81c306
```

This command is source-pinned and read-only. A later candidate needs its own
baseline/source identity; do not change these pins to make old evidence pass.
