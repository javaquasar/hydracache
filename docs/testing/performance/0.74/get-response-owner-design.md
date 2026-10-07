# Private GET response ownership transfer: D2 preregistration

This is one new proposal, `p74-get-response-owner-v1`, not a rescue of rejected
serial scratch or a reopening of W4/W8 terminal negatives. Baseline is clean
`36f60265`; D1 source `2562f6f7` attributes one payload-sized borrowed-reducer
allocation for each nonempty GET in all three repeated screens. D1's dispatch
counts exclude encoding/translation and are not candidate improvement claims.

## Narrow implementation

Add off-by-default `experimental-resp-get-owner-074` with no dependency features.
The canonical executor still translates, clones initial requests, dispatches,
collects responses, validates/executes followups and increments errors at the
same boundaries. Only its final reduction selection is enrolled. A private
helper receives the existing owned response vector, inspects the plan and
response by reference, and transfers a value only if all predicates hold:

- exactly one initial `ClientRequest::Get`;
- GET reducer and no followup plan;
- exactly one actual response;
- successful `ClientResponse::Value { value: Some(nonempty_vec) }`;
- response Vec capacity equals its length (no extra capacity carried forward).

This final predicate is a pre-implementation tightening of `bf42f776`, not a
changed numerical floor. Transferring a Vec with spare capacity could keep more
capacity alive during encoding than the canonical length-sized clone. Such a
response must fall back. A separate baseline fixture confirms canonical GET
returns length-sized vectors for the registered local payload sizes. A private
synthetic spare-capacity test must prove fallback even if a future backend changes.

Every other case calls the existing public borrowed `plan.reduce(&responses)`
without modifying responses. That includes empty/missing values, errors, wrong
result kind/count, and all other reducers (including multi-key and TTL). The
helper adds no protocol-version check; response versions and authorization
were already handled by canonical dispatch, and reduction preserves the old
contract rather than inventing a new wire boundary.

Only after admission does the helper take its one response's owned Vec, leaving
an empty Vec in the soon-to-drop envelope, and move it into `RespValue::BulkString`.
No copy, mutable shared backing storage,
borrowed payload or response scratch survives into the writer. The response
vector and request metadata release normally. Encoding and per-command
`write_all`/`flush` are unchanged, as are malformed/AUTH/HELLO/QUIT/subscription
barriers, command order, expiry, accounting, durability and native code.
There is no connection-retained capacity or cross-command reuse. A response
already materialized before a slow write remains that immutable response even
if another native client replaces/expires/deletes the store value.

The public `RedisExecutionPlan::reduce(&[ClientResponseEnvelope])` continues to
clone borrowed GET payloads and remains safe for callers retaining responses.
No public Rust API, durable record, wire format or configuration changes;
COMPAT needs no new format entry. Rollback disables/removes this feature only.

## Baseline and tests before implementation

`tests/resp_get_owner_074.rs` first records the canonical public API and execution
semantics against a separately executed borrowed-plan oracle. It covers binary,
empty, missing, 64-byte, 4-KiB and 1-MiB values; wrong response counts/kinds;
errors; RESP2/RESP3 bytes; a retained response after replacement/expiry; and
multi-key duplicate/order/count semantics. It is committed and run on the
unmodified product baseline before the helper is implemented.

Private tests then compare the actual consuming helper with the independent
borrowed reducer, checking identical results/error details, distinct borrowed
clone pointer and identical transferred pointer. A fixed-seed property corpus
includes arbitrary bytes and misses. Synthetic invalid plan/response cases
exercise fallback rather than testing only well-formed server GETs.

Existing adversarial tests run both feature-off and feature-on to prove partial
writes, pending/failed flush, disconnect/cancellation, fragmentation, large
payloads, native progress, exact expiry, tenant/quota/auth and queued-mutation
frontiers. Those tests' one-byte read controls are canonical **writer** controls;
they do not force a borrowed reducer for this candidate. Private borrowed-oracle
tests provide that independent reduction comparison. Server lifecycle tests
run both builds. Package/direct-dependent check, strict lint, format, contract,
documentation/governance and expected-red canaries remain required.

The existing canonical runtime fingerprint guard is intentionally extended
before product mutation: it strips only the two exact allowlisted cfg hunks
(module declaration and final reduction selector), reconstructs the old final
`match plan.reduce(&responses)` and checks the same full LF-normalized hash.
It rejects unknown edits rather than removing the baseline fingerprint. This
proves the experiment is confined to its owner; it is not a numeric feature-off
or native nonregression receipt. Historical negative packets remain untouched.

## Numerical boundary

The proposal contract authorizes D2 source/tests only. No candidate measurements
are authorized by it. Before D3, a new finite screen contract must seal clean
source/feature/binary/tool-lock/toolchain/workload identity and every attempted
pair. The unchanged allocation floor is 20% **end-to-end** for large GET; private
reducer pointer sharing cannot satisfy that floor by itself. At least five
counterbalanced independent pairs and AA controls are required. Peak-live must
not increase; next-read/post-close owner increments must not increase.

All four native surfaces remain independent guards, alongside timing, true
transport/TLS/concurrency and idle/RSS/allocator-retention/refill checks before
acceptance. Tool allocator timing is never product CPU/p99 evidence. Neither
source inspection nor unchanged native source certifies a numerical native
guard. No costly workflow, rented host, 6-hour/24-hour campaign or qualification
manifest change is part of this local D2 step. Accepted proposals stay zero and
C74 unresolved until complete qualifying evidence exists.
