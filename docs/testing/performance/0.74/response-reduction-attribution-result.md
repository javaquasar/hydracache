# GET response reduction: local D1 owner attribution

## Decision and identity

Classification: **owner attributed, not a product optimization**. Source and
preregistration are `2562f6f7e2ff598741d4fe9a4f38ae635786e8d9`. There is no candidate
feature or runtime mutation. The rejected serial scratch remains removed and its
140-attempt negative archive is unchanged. Accepted product proposals remain zero,
C74 is unresolved, and ship admission remains closed.

The independent contract is
[response-reduction-attribution-contract.toml](response-reduction-attribution-contract.toml).
All 18 fresh-process attempts succeeded without retry or sample selection: six
fixed cells, three rotating repeats. Full raw/attempt/seal/summary receipts are
in `local-runs/response-owner-2562f6f7/` (38 files). Release binary SHA-256 is
`7ba6f151ce67c9073e9b9a5e9a4501377b3f6be1d6122d2cdac0b228f99ee6b4`;
tool-lock SHA-256 is
`93b11f444aa2b8d0a21f083ae4629188b67d84c8d7861c3ea57c8c3161571fd5`.
The seal retains contract hash and Rust 1.94.0 / Windows MSVC toolchain identity.
These tool dependencies are not the root product dependency lock: the tool lock
inherits the existing attribution versions, with only its root package changed.

## Hypothesis and measurement boundary

Canonical `execute_plan` collects verified dispatch responses, validates followups
and passes a borrowed response slice to `RedisExecutionPlan::reduce`. `reduce_get`
clones `Option<Vec<u8>>` into `RespValue::BulkString`. A successful nonempty GET
therefore has an original response value and a newly allocated reduced value
alive together. Source inspection suggested this owner; D1 measures its magnitude
separately from the already measured encoder allocation.

Three windows use public, unchanged product APIs:

1. Dispatch-only clones the prebuilt request envelope, collects the response vector
   and validates followups. It includes ordinary client validation/store work.
2. Reducer-only borrows a prebuilt response outside the window, validating exact
   bytes and distinct nonempty backing pointers while the original remains alive.
3. Dispatch-and-reduce repeats the first chain and canonical borrowed reduction
   using an independently and identically initialized surface.

The plan, binary key, seed-740074 value corpus and expected results are constructed
outside all windows. Successful validators allocate nothing, as an exact counter
test proves. Each dispatch control has 100 warmup operations. A deterministic
cache clock is injected at 1,000,000 ms; no expiration or production-clock CPU
claim is made. Each GET control reconciles preload plus warmup plus measured
dispatches, one preload mutation for hits, exact cardinality and a final read.
SET reconciles every mutation. The miss surface stays empty.

There is no decoder, command translation, server request-id generation, encoder,
socket, pipeline scheduling or wait inside this owner screen. The plan is reused
only by the measurement tool, not by product execution. Consequently these
dispatch totals are not the earlier full-server/stage totals and cannot be
subtracted across those different workloads to manufacture a candidate gain.

## Repeated results

All three repeats have identical stage gross/call/live-increment totals. Each
number below is therefore both the per-cell minimum and maximum, not a best run.
Bytes/op count gross successful requested layouts. Peak counts outstanding
requested layouts above that window's initial owners, not RSS or allocator arenas.

| Cell | Dispatch B/op | Reducer B/op | Combined B/op | Reducer calls/op | Dispatch peak B | Combined peak B |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| GET empty hit | 336 | 0 | 336 | 0 | 336 | 336 |
| GET 64 B hit | 400 | 64 | 464 | 1 | 400 | 400 |
| GET 4 KiB hit | 4,432 | 4,096 | 8,528 | 1 | 4,432 | 8,294 |
| GET 1 MiB hit | 1,048,912 | 1,048,576 | 2,097,488 | 1 | 1,048,912 | 2,097,254 |
| GET miss | 336 | 0 | 336 | 0 | 336 | 336 |
| SET 4 KiB | 4,432 | 0 | 4,432 | 0 | 4,432 | 4,432 |

GET dispatch has 11 allocation/reallocation calls for nonempty values, combined
has 12; empty/miss have 10, SET has 11. These are successful allocator calls,
not proof of the number of machine copies. The isolated reducer's peak is exactly
one payload (64 / 4,096 / 1,048,576 bytes), and the combined-minus-dispatch gross
total equals the isolated reducer total in every sample: no unassigned residual.

For 4 KiB, the live checkpoint after dispatch is 4,198 bytes above start, and
after reduction it is 8,294. For 1 MiB it is 1,048,678 then 2,097,254. The
checkpoint difference is exactly one payload. However, the *window peak*
difference is 3,862 / 1,048,342 bytes because dispatch itself temporarily owns
234 more metadata bytes than its completed-response checkpoint. At 64 bytes
the dispatch peak still dominates, so an extra live owner does not increase the
whole-window peak. Stage peaks must not be added or their maxima confused with
owner checkpoints.

Every window ends at its starting outstanding-layout count. That establishes
owner release in this synchronous tool, not timed idle retention or physical
memory return. The public client's own value materialization remains in dispatch;
this attribution does not remove it or reopen the stable-protocol N6 decision.

## What follows, and what does not

The new owner justifies designing one independent D2 hypothesis: move a successful
GET response value into the private execution result instead of cloning it,
while keeping the public borrowed reducer API unchanged. It must not add scratch
reuse, output batching, a decoder shortcut, native representation changes or
store changes. This D1 contract does **not** authorize that product mutation.
A separate pre-candidate D2 contract must fix exact fallback/error handling,
single-response validation, protocol/version behavior, memory lifetime, tests,
allocation floor and unchanged native/peak/idle guards before implementation.

The measured payload owner is not a promised percentage reduction in total
RESP allocation: omitted decoder/translator/encoder owners still count in an
end-to-end comparison. Fewer payload copies do not prove throughput, CPU/op,
p99 or native nonregression. D3 needs at least five counterbalanced independent
pairs, exact semantic/transport checks and all four native surfaces separately.
No expensive host/qualification work, service operation or frozen 0.73 mutation
occurred here. No qualification manifest, threshold or product dependency changed.

Two read-only documentation lookup paths were absent during development and were
corrected by repository inventory. PowerShell reported an existing empty archive
directory while copying the completed packet, but all 38 files copied; bytewise
hash verification and complete replay confirm the archive. Neither diagnostic
started/retried a workload or changed any measurement.
