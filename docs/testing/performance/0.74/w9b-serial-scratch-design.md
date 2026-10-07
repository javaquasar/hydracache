# W9b D2: bounded serial GET encoder scratch

## Decision and identity

The large-GET owner identified in `w9b-response-buffer-attribution.md` supports
one local implementation experiment, not default activation or acceptance. The
preregistered policy/guards commit is `4c5b16d77adf77a5c857ef5b04002dbdcf894bac`;
baseline is `4d733e3189bdc777635792795e1bbb58ab00a7cb`; implementation is
`b8cc7c6c32afb6e4f7fa1efaf92e14846f7102b1`. The proposal policy now pins that
implementation without changing thresholds. C74 is still `UNRESOLVED`, accepted
product proposals zero, and admission closed. Earlier W3/W4/W9 negative evidence
is unchanged; this is a separately identified large-response owner follow-up.

Feature `experimental-resp-serial-scratch-074` is off by default and is not an
accepted release feature. No public configuration or codec function changed.
`--all-features` selects this experiment; it is not a production activation
recommendation. Default builds keep the original writer branch. This remains
not distributed transactions and changes no durability/acknowledgement contract.

## Mechanism and lifetime

A private `SerialGetScratch` exists only inside one received-read iteration. It
inspects the actual already executed response, never a predicted result or a
lookahead command. Only successful GET bulk values from 4 KiB through 1 MiB are
eligible. The first GET uses the original `write_response`; the second consecutive
equal-sized GET allocates scratch; third and later equal-sized GETs reuse capacity.
This deliberately gives up savings on the first two responses to avoid a second
decode or changing the shallow first-response path.

The scratch encoder delegates to the pinned RESP2/RESP3 frame encoder. A pointer
regression demonstrates reuse with exact binary payload/frame bytes, including
replacement by zero, high-bit, CR and LF bytes. Equal sizes do not grow capacity;
size/shape changes drop the prior buffer rather than retaining the largest-ever
reply. Capacity is bounded at 1,048,588 bytes (1 MiB bulk frame with header/CRLF).
An internal misuse or unexpected dependency capacity departure fails loudly.

Every response still has its own awaited `write_all` and awaited `flush`, with
the original attribution counters. The next command is decoded/executed only
after that boundary. AUTH, HELLO, subscription, mutation and QUIT commands drop
scratch before execution; malformed input drops it before its error reply. A
miss/error or response-size change drops it before canonical encoding. At the
end of the read iteration the owner drops before the next read/event/idle await.
EOF/error/QUIT/cancellation also drop the connection future's owned buffer.
There is no server-owned buffer, pool, shared lock, native-path atomic, borrowed
command across IO, reservation, speculative mutation or compensating rollback.

The bound/lifetime proof does **not** establish total peak-live-memory or RSS
non-regression. Retained scratch now overlaps later canonical execution/reduction;
those transient owners may increase peak memory even when total allocated bytes
fall. That risk must be measured explicitly in D3. Close drops the Rust owner;
allocator arenas/pages need not immediately return to the OS.

## Tests and source-bound checks

Tests were first run on the canonical baseline. The implementation adds four
private unit guards and a 32-case binary property test. Integration now blocks
the third GET, when scratch is actually reused, at zero/one/midpoint/last/full
reply cuts for both 4 KiB and 1 MiB. Native PUT progresses while that response
is pending; queued SET stays unapplied. Separate write/flush errors and
cancellation leave that later mutation absent. Five emitted responses still
require five writes and five flushes, with unchanged response high-water bytes.

Further guards compare one-byte fragmented canonical-writer controls with complete
pipelines through AUTH, subscription, malformed input and QUIT. Mixed sizes and
HELLO transitions preserve GET/miss/QUIT bytes; the existing RESP3 HELLO hash map
is compared structurally because its wire entry order is unspecified. Injected
exact expiry while the third GET is blocked makes the fourth GET a miss. A native
PUT can claim remaining tenant quota during the blocked GET; subsequent SET
returns the exact quota error and one rejection audit, without a third mutation.

Clean implementation source passed 166 default RESP tests and 171 feature-on
tests, with 23 existing opt-in cases ignored in each build; none are counted as
executed. Eighteen feature-on Redis server lifecycle tests pass (nine unrelated
cases filtered). All 94 targeted xtask tests pass: 42 contract, 13 evidence,
23 governance, 16 documentation. Affected/default/candidate checking, strict
all-target/all-feature lint including the direct server dependent, format, the
local non-promotable contract, 17 governance checks and doc registry pass.
W11/W12 canaries remain expected red. The pipeline profiler also compiles with
the dependency's candidate feature, without executing a candidate workload.

Source hashes, exact commands, scope limits and initial fixture/lint/Windows
executable-lock diagnostics are retained in
`local-runs/w9b-serial-scratch-semantic-b8cc7c6c.json`. The Windows failure was a
validation relink conflict with a still-running canary, not a failed product test
or restarted performance attempt. Checks subsequently ran sequentially on the
same clean source. No hosted-CI receipt or full-workspace milestone is claimed.

```powershell
cargo test -p hydracache-redis-compat --locked
cargo test -p hydracache-redis-compat --features experimental-resp-serial-scratch-074 --locked
cargo test -p hydracache-server --test server_lifecycle --features hydracache-redis-compat/experimental-resp-serial-scratch-074 --locked redis
```

## D3 boundary and remaining risks

No numerical optimized candidate run has started. Gross allocation reduction,
goodput, p50/p95/p99, CPU/op, RSS, retained memory, copied bytes and actual syscalls
have no new improvement claim. Keeping a buffer address stable is ownership
evidence, not an end-to-end allocation receipt. Payload copying and every
write/flush remain; no syscall-reduction claim is appropriate.

Before D3, add explicit source/feature/binary identity to the harness receipt and
seal a matched feature-off/on matrix with independent AA controls. Use at least
five counterbalanced pairs and unchanged affected end-to-end allocation floor
0.20, unaffected goodput floor 0.98 and CPU/p99 ceiling 1.03. Embedded,
ClientSurfaceState, HC1 and HC2 must remain separate controls. Register and execute
the exact feature-on gate before promotion: ordinary fast workspace tests alone
do not exercise the private feature-only unit/property code.

Include pipeline-one, small GET/SET, 4 KiB/1 MiB GET, deep/concurrent pipelines,
mixed payloads and response-size changes; distinguish plaintext/mTLS and errors.
Measure active/peak and post-read idle memory explicitly. Fragmentation can end
reuse early; small outputs, heterogeneous responses and shallow pipelines may
provide no benefit. If a semantic, memory or unaffected/native guard fails, retain
the negative result and remove only this candidate, never tune the floor or add
a reducer/native/allocator optimization to rescue it. No rented-host or expensive
qualification run is authorized by this local D2 completion.
