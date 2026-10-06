# W3 staged-execution architecture assessment

Status: assessment complete, staged batching not admitted for implementation. This is an authorized
local design investigation following the delivery-frontier review, not a D2 product proposal,
benchmark candidate or modification of the previous W3 dispositions. Baseline is `571cd070`.

## Question and scope

Can a bounded connection-local prepared overlay make a two-reply write possible without executing
the second mutation before the first response's complete write and flush? The question includes
native progress, time/expiry, quota, exact response bytes and fail-loud behavior. An unchanged
success byte string alone is not unchanged semantics. This is still **not distributed transactions**;
no cross-node atomic commit, store redesign, new public API, persistence mode or wire format is
proposed. The published 0.73 runtime and qualification identities remain untouched.

## The state machine that a compatible executor must retain

The first command executes canonically and is visible even if its response write subsequently
blocks or fails. The second command may be parsed/prepared locally, but its execution remains
ineligible until the first complete response write and flush succeed. At eligibility, identity,
namespace/policy, current time, live key state and quota still have to be validated through the
canonical seam. Only a successful state transition can supply its success response and normal
ordered events/counters. A failed admission supplies the original error and audit instead.

The compatible sequence is:

```text
execute/commit command 1 -> encode/write reply 1 -> successful flush 1
  -> validate current state/time/admission -> execute/commit command 2
  -> encode/write reply 2 -> successful flush 2
```

"Write and flush" means the supplied `AsyncWrite` boundary, not peer receipt, TCP ACK or durable
persistence. Changing the writer to a buffer that calls application acceptance delivery would
replace that boundary. Flush behavior cannot be assumed identical between generic, TCP and TLS
writers merely because one transport may implement a no-op flush.

Preparation must not mutate shared state, reserve tenant quota, publish events/audit, pin a store
lock across IO or cache a success result as authoritative. Even a local immutable snapshot cannot
remove the need to revalidate. If preparation is ever proposed separately, command dialect/auth
transitions, malformed-frame timing, QUIT/subscription/drain barriers, buffer limits, credential
lifetimes and cancellation must also remain exact; no such preparation candidate is authorized here.

## Real canonical counterexamples, not a speculative implementation

Four integration tests explicitly poll a RESP serving future with two commands and QUIT already
in one read. The writer stops at every cut of the first five-byte `+OK\r\n`: zero bytes, each
partial prefix, or all bytes with pending flush. Intervening operations call the supported direct
`ClientSurfaceState::dispatch_verified_request` seam. Translation is used only to obtain the
existing binary-safe key identity, not to execute those native operations.

| While reply 1 is pending | Required result when command 2 becomes eligible | What early staging gets wrong |
| --- | --- | --- |
| Native invalidate removes the key after SET 1 | `SET ... NX PX 5000` succeeds and stores value 2 | A snapshot of the first value predicts NX failure. |
| Native put replaces the first value with `N` | Queued GET returns `N` | A captured read response returns the old value. |
| Injected clock reaches the first key's exact expiry | Queued NX succeeds; its new 10 ms TTL starts at execution | A captured condition/time can reject NX or shorten the replacement TTL. |
| Native put consumes the remaining byte/entry quota | Queued SET returns the exact quota error, without mutating its key | Early reservation harms native admission; predicted success bypasses quota. |

Each test covers all six cuts: 24 controlled trace instances. These are deterministic semantic
cases, not statistical samples, throughput tests or a native latency bound. Successful native
operations complete while the original writer remains pending. This proves progress for the
enumerated schedule, not general lock freedom or the complete concurrent native policy surface.

The quota case verifies two exact committed entries, no third SET mutation, and one `QuotaRejected`
audit event. The expiry case advances only the injected clock, checks the replacement immediately,
at nine milliseconds and at its exact ten-millisecond expiry; it never waits for real time to pass.
The tests use the supported `NX` plus TTL form. An initial development fixture used unsupported
bare NX and correctly received the existing syntax error; the fixture was corrected, not the server.

Authorization revocation, policy generation, event delivery, idempotency, cancellation and atomic
multi-key mutation remain additional obligations for any future executor. This assessment does not
claim that its four counterexamples independently exhaust those obligations.

## Fixed shared-write ordering model

The finite test model assumes SET 1 has committed and one **fixed, pre-encoded** write contains
both response frames. Let `B` be that write's start, `F1` completion of reply 1's flush, and `C2`
commit of SET 2. A valid chronological shared write requires `B < F1`. The retained frontier
requires `F1 < C2`. Exposing a prepared success frame without speculative acknowledgement requires
`C2 < B`: the generic writer may accept every supplied byte immediately. These three constraints
cannot hold together.

The test enumerates all six event permutations; three have a chronological batch write and none
preserve both guards. A positive control admits the canonical `F1 < C2 < begin-write-2` sequence;
it uses a separate second response write. The model is an executable check of stated assumptions,
not a proof about every transport, dynamically supplied buffer or future executor.

## Options and decision

- **Commit both mutations before a shared write:** violates the mutation frontier immediately.
- **Write predicted successes, then commit:** a partial write can expose success before commit;
  admission/IO/cancellation failure cannot retract bytes. It also delays the first command's
  visibility if its commit was postponed, contradicting the canonical blocked-writer state.
- **Connection-local overlay without shared commit:** avoids early visibility but not stale results;
  the native, expiry and quota counterexamples require post-flush revalidation and a later reply.
- **Lock the store or reserve quota until delivery:** stabilizes a snapshot by changing native
  progress/admission and tying shared capacity to a slow reader. This transfers cost to native.
- **Apply then roll back after failure:** cannot undo an observed value, event or audit, and a blind
  restore can overwrite an intervening native mutation. No such rollback engine is proposed.
- **Prepare locally, then validate/execute after flush:** can preserve semantics in principle, but
  the first write has already happened and the final second reply still needs another write. This
  supplies no fixed-batch syscall reduction and has no measured CPU/allocation candidate here.
- **Dynamic/transport-specific writer:** not settled by the fixed-batch model. Renaming several
  underlying writes as one API call is not kernel syscall reduction. A distinct writer would need
  a full completion/partial-write/TLS/error proof and actual counters before D2, not an assumption
  that writability permits another command.

No reviewed staged design supplies the requested write/flush reduction while retaining the frozen
guards. Therefore do not implement a speculative mutation engine, alter acknowledgement or relax
backpressure for 0.74. Retain the existing W3 negative disposition. This conclusion is narrower
than "all output optimizations are impossible": a genuinely different owner/mechanism could be
reviewed later, but these counterexamples must remain part of its admission screen.

No runtime/native/embedded path, dependency or resource limit changed. Deep/shallow thresholds are
copied unchanged into `w3-staged-execution-review.toml` and checked against the previous contract.
No product comparison, rented-host workload or expensive qualification was run. Accepted proposals
remain zero, C74 remains unresolved and release admission stays closed; documentation is not C74.

## Local verification

```powershell
cargo test -p hydracache-redis-compat --locked
cargo check -p hydracache-redis-compat --all-targets --locked
cargo clippy -p hydracache-redis-compat --all-targets --all-features --locked -- -D warnings
cargo test -p xtask --test performance_contract_074 --locked
cargo xtask performance-contract-check --release 0.74
```

Concrete tests are the four `queued_*` functions in `resp_transport_adversarial_074.rs`, the finite
`fixed_two_reply_batch_cannot_preserve_both_commit_frontier_and_response_order` model in
`resp_staged_order_model_074.rs`, and the admission/policy regression
`w3_staged_execution_assessment_does_not_authorize_product_or_threshold_changes` in xtask.
