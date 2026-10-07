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

## D2 implementation and local semantic result

The preregistration was committed at `bf42f776`, the capacity admission tightened
before product mutation at `d07c35f0`, and the private helper implemented at
`2379698d`. CI enrollment/its exact-command guard are separate at `b3f9d2c4`.
The immutable proposal remains the before-mutation policy; current implementation
status lives in the proposal registry. `candidate_source_sha` stays unresolved
until a distinct D3 measurement source is sealed. There were no D3 attempts.

The implementation uses `std::mem::take` only after all eligibility predicates.
Seven private tests exercise pointer transfer, spare-capacity fallback, every
client error code, invalid counts/plans, all non-GET reducers, envelope metadata
and 128 fixed-seed property cases. Five integration fixtures compare execution
with a separately executed public borrowed-plan oracle and test retained response,
capacity, expiration and multi-key duplicate/order/count behavior.

The unchanged baseline passed 170 RESP tests before the capacity fixture (and all
five fixtures after it). Implementation passed 178 tests in each default and
feature-on build, with 23 existing opt-in tests ignored in each; all 19 adversarial
transport tests passed in both. Each server build passed 18 Redis lifecycle cases
(nine unrelated cases filtered). Affected packages/direct server and xtask passed
all-target check and strict lint. The canonical normalized runtime fingerprint
remains `c98ab563d4fc9181735d432484118561ad9bf773d32fb2865f4a591dcc50e932`
after removing only the two enrolled overlay hunks. This is source isolation,
not native numerical nonregression.

One initial property assertion wrongly presumed every generated Vec had
length-sized capacity. At seed 740074 the helper correctly fell back for spare
capacity; the test failed. The oracle was corrected to honor the already sealed
predicate and explicitly generate both branches. Product logic, seed, numerical
floors and workload did not change. This development diagnostic is retained in
the semantic receipt, not omitted or presented as a performance retry.

Pointer identity establishes that an admitted private result owns the original
payload. It does not establish whole-command gross allocation, lifetime peak,
RSS, CPU/op or goodput improvement. Next: seal a finite D3 A/A and counterbalanced
A/B early screen before any sample, keeping the 20% large-GET allocation floor,
zero peak increase and complete attempt retention. Only if it passes proceed to
independent native, unprofiled timing, real transport/security/concurrency,
retention/refill and hosted feature-on evidence. The rejected scratch packet and
all previous terminal decisions remain intact. Qualification manifest, root lock
and frozen 0.73 are unchanged; ship admission remains closed.

Clean `3f13f1d1` subsequently produced W11/W12 ExpectedRed receipts (3,499 and
54,042 ms), retained in `local-runs/get-response-owner-checks-3f13f1d1.json`.
Local aggregation resolves all 28 structural source/test/artifact rows, but has
zero fast-green, gated-green or ship-ready rows without complete lane receipts.
It is not an accepted optimization count or release readiness. Before that
evidence commit, 99 targeted xtask, 16 replay and three documentation-script tests,
format, contract, 17 governance checks, docs registry/sync/links and mdbook passed.
Full workspace verify and hosted CI are not claimed by these local checks.

## D3a preregistered early screen

`get-response-owner-d3-contract.toml` seals nine local cells and 180 fresh-process
attempts before data: five A/A pairs, then five alternating A/B pairs for each.
The new standalone `resp-get-owner-screen-074` workspace forwards only the new
default-off product feature. Both release binaries must be built from the same
clean instrumentation SHA. The allocator/build receipt are reused by source
reference; the retired scratch tool, flag, contract and packet are unmodified.

The complete canonical connection path runs against immediately-ready scripted
RESP2 IO. Precomputed requests/expected replies are outside counting; there is no
output collection in the window. Every write validates exact bytes. Preload,
five warmup batches, final value/cardinality validation and receipt serialization
are outside the epoch. Dispatch/mutation/error counters are read outside the
window and checked against exact expected counts. The production clock is not
replaced; these cells have no TTL-bearing workload. Duration is identical fixed
work, not an elapsed-time throughput estimator.

The 20% gross allocation floor applies to large-GET primary cells; the remaining
cells enforce at most 5% gross increase. Their `affected=false` labels mean only
that no 20% floor is demanded, not that eligible small/fragmented GETs cannot
transfer. Empty hits, misses and SET are actual canonical fallback controls.
All cells must show no higher peak above start or next-read/post-close owner
increments. All five paired ratios and their log-scale t(4) intervals survive;
independent A/A gross/peak noise above 1% invalidates rather than changes policy.

The runner seals binary hashes, source, contract, tool lock, compiler and finite
schedule. Raw hashes accompany every retained attempt; offline replay requires
all 362 files, exact order, trace and compiled variant. Invalid attempts stop
without retry; valid red still completes the sealed matrix before selective
rollback. Passing only permits preparing a separately sealed phase B contract,
not acceptance. RSS endpoints/lifetime peak, scripted write calls and requested
layouts cannot establish timed retention, actual syscalls, native CPU/p99/goodput
or real secure/concurrent transport guards. No costly workload is authorized.
