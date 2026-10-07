# Scheduled operation latency and independent native controls

This is local instrumentation/semantic coverage for the private GET owner, not a
numerical cohort, B0 retry, native nonregression result or full D3 admission. The
feature remains default-off; accepted product changes remain zero and integrated
C74 unresolved. B0's refused launch and D3a's sealed tools/data are unchanged.

## Question and measurement boundaries

Closed-loop latency can hide queueing during a stall: delaying the next request
until a previous response arrives reduces offered work precisely when the system
is slow. Pipeline batch time divided by depth cannot recover response tails.
The new driver keeps each offer's original fixed-rate timestamp, actual execution
start and terminal timestamp. Scheduled response latency includes generator and
queue delay; service response latency starts only when the target is invoked.
Both are retained separately. No send-time rescheduling, skipped offers, implicit
retry or success-only denominator is permitted.

`tools/get-owner-scheduled-controls-074` source-references the existing loadgen
schedule, Target and legacy histogram modules, rather than introducing another
calendar or linking its process-wide counting allocator. The imported legacy
`run_open_loop` is not called: its unbounded queue/task ownership is replaced by
this bounded extension. Default allocator and disabled product stage counters
do not make observation overhead free. Timestamp slots, mutexes, task ownership,
byte oracles and histograms are tool costs; future paired measurements must hold
them identical and label the measured boundary explicitly.

## Bounded ownership and complete accounting

The local limits are 10,000 offers, concurrency 1/8/32/128, at most 1,024 queued
owners, a 15-second calendar and at most five seconds each for operation timeout
and drain. A semaphore bounds executor slots; the JoinSet never exceeds slots
plus queue allowance. A full queue records admission rejection without creating
an executor. Queue deadlines are measured from original offer time; expired work
is recorded as queue timeout without receiving a fresh execution budget.

Every offer retains one terminal class: success, error, execution timeout, queue
timeout, target rejection, admission rejection or incomplete. Completed executor
responses populate separate scheduled/service histograms, including failed
responses. Queue/admission failures have no fabricated response latency.
Incomplete work has a censored lower bound through cancellation/drain and does
not populate a response histogram. Response p99 must always travel with all-offer
loss counts; it is not the p99 of all offered work. Values beyond histogram bounds
are clamped only for recording and counted explicitly as overflow; future
admission must reject precision loss rather than hiding it.

Good successes satisfy the configured scheduled-latency SLO. Goodput divides
them by the elapsed original-offer-through-owned-task-drain interval; good
fraction divides them by all offers. These are diagnostic definitions, not
activated release thresholds. Projection rejects timestamp/order drift, missing
starts, conflated boundaries, invented incomplete responses and undrained owners.
Drain aborts and joins unfinished requests, preserving completions that won the
abort race. Panics are loud errors with owned tasks drained. Caller cancellation
aborts owned request tasks; the outer fixture must still shut down transport
owners explicitly.

## Independent HC1 and HC2 transport controls

HC1 uses the production axum route owner and real HTTP/1 loopback requests with
explicit client/tenant headers. HC2 uses the production listener, generated gRPC
adapter and real mandatory mTLS handshake. Dynamically bound loopback sockets
avoid port reservation races. No daemon child, host service, workload campaign,
clock override or store-reset shortcut is used. Ephemeral certificates/private
keys stay in temporary fixtures and are not evidence artifacts.

Controls have independent ClientSurfaceState instances. The same seed 740074,
binary keys, fixed values, keyspace and digest are used in either surface; HC1
mirrors HC2's namespace/hex key mapping. Preload and final GET/missing-GET byte
oracles traverse the same real transport. PUT writes the same fixed value, not a
random mutating distribution; this restriction must be retained in future cell
identity. Slots serialize each client with a tool-only mutex. This is logical
client concurrency, not proof of a particular physical HTTP socket count.

HC1's fixture-only frame ceiling is explicitly 8 MiB so a 1 MiB value plus
envelope fits; product defaults are not modified. HC1 HTTP header identity and
HC2 verified mTLS identity are different security contexts. Their numeric results
must never be pooled or compared as security-matched transports. Future off/on
controls must compare each surface independently with identical policy/lock.
HC2 deadline and quota failures preserve timeout/rejection classification; HTTP
transport timeout is timeout, other HTTP/envelope failures remain errors. Neither
client retries. Explicit shutdown closes clients, joins the listener within a
bounded safety budget and checks HC2 live-resource accounting. Drop aborts the
owned listener on setup/test failure; this is a safety net, not a clean-drain
receipt.

## What is tested, and what remains unproved

Virtual-time tests demonstrate stall queueing, overload, original queue deadlines,
drain censoring, panic/caller cancellation, all-offer conservation, forged
projection rejection, histogram overflow and bounded 1/8/32/128 logical slots.
Real native fixtures cover GET/PUT at 1/8 slots, binary values, missing keys,
1 MiB frame round trips, anonymous HC1 rejection, tenant separation and foreign
HC2 CA rejection before dispatch. No real-clock percentile/speed assertion or
numerical comparison is made. Compilation initially exposed a test mistakenly
treating exactly 15 seconds as over-budget; the test was corrected to 16 seconds,
without changing the driver limit.

The scheduled RESP adapter still needs exact per-response timestamps, pipelined
offer/order correspondence and fragmented/partial-write/slow-reader coverage.
Real native transport concurrency 32/128 is also not yet a measured/covered full
grid. Matched mTLS/RESP3, representative miss/error/size transitions, allocator
active/resident/retained, timed idle/refill and current-source feature-on hosted CI
remain required. No CPU, allocation, copied-byte, syscall, lock-contention or RSS
measurement is supplied by this library; do not infer those from its latency
output or retained entry/value counts.

Before any numerical series, commit the instrumentation and separately seal the
finite cohort, binaries, source-referenced files, dependency lock, host placement,
offered rates, SLOs, duration and unchanged native regression/noise guards. Preserve
the invalidated B0 packet; do not retry it under this profile. Only after complete
paired evidence may the nonregression question be answered.

## Retained focused verification

At clean source `a6047895509f8298f5c9154d9d508bc0eed92f26`, all 19 tool tests
passed in each default/get-owner build, as did all-target/all-feature check and
strict clippy. The 51 performance-contract, 13 release-evidence, 23 governance and
16 doc xtask tests passed, along with xtask check/clippy, local performance/doc
checks, 17 governance checks, three documentation script tests, link/sync checks
and mdbook build. See the non-promotable
[focused check summary](local-runs/get-owner-scheduled-instrumentation-checks-a6047895.json)
for commands and source-reference/lock Git blob ids. These are functional check
counts, not a numerical performance result, current-source hosted receipt or full
workspace verification. Qualification manifest digest remains unchanged.

## RESP response matching and the high-concurrency semantic extension

The next tool-only extension adds RESP2 GET on one real TCP connection through
the production `RedisRespServer::serve_connection` owner. It uses the same binary
Dataset as native controls, but no security-matched cross-surface comparison is
claimed. Preload SET and final GET/miss oracles use the real transport, not direct
store injection. The fixture-only request frame ceiling is 8 MiB, because a 1 MiB
SET value plus envelope exceeds the production default 1 MiB frame ceiling. The
first large-payload fixture found that limit; only this local fixture config was
corrected. Product defaults, threshold policy and old B0/D3a tools remain intact.

Each accepted request retains its caller sequence and actual wire FIFO ordinal.
The single owned I/O actor interleaves partial writes and reads, with bounded
channel/semaphore ownership, a one-frame reply buffer and at most 10,256 history
identities (10,000 offers plus a bounded setup allowance). It supports only the
bulk/null/simple/error RESP2 reply forms used by this control and rejects unknown
types, malformed lengths, oversized payload/header or unsolicited responses.
Header-boundary tests also cover CR and LF fragmented exactly at the limit.
Outstanding ceilings 1/10/50 are NOT fixed batch sizes, connection counts or
permission to divide a batch duration by depth. Actual wire order is retained;
concurrent driver wakeups do not guarantee ascending sequence order on the wire.

RESP has no request id. Aborting a waiting caller does not undo bytes already
written, and deleting its FIFO entry would assign its late reply to another key.
The actor therefore owns the slot until reply or close, even when the waiting
future is cancelled. Its tombstone records reply kind, exact-byte verdict and
whether the receiver was closed. Disconnect/parse failures have no fabricated
frame timestamp. Distinct scripted replies prove cancellation does not shift
response association. A logically gated slow reader and partial-write duplex
fixture prove the bound remains owned; another fixture proves reads progress
while a later command write is backpressured. These are deterministic I/O
fixtures, not cluster chaos or wall-clock speed assertions.

`scheduled::run_at` shares the original monotonic origin with the sidecar. Each
wire sample carries original scheduled time, slot acceptance, full write and
complete-frame-observed time. The last is a userspace parsing observation, not a
kernel last-byte-arrival timestamp; coalesced reads are still parsed per frame.
Byte-oracle validation and waiter wakeup occur afterward. The driver's verified
operation histogram keeps that later boundary. Frame latency is separate
attribution and never replaces verified-operation SLO/goodput. Every sidecar
sample also carries the driver's terminal outcome: a parsed late reply cannot
turn a timed-out/incomplete caller into success. The validator rejects duplicate
or orphan identities, changed schedule/FIFO order, absent successful replies,
inconsistent write/response times and fabricated wire responses.

After driver request-task cleanup, the control drains wire FIFO owners within
five seconds; inability to drain is a loud error, not a clean receipt. Elapsed
goodput accounting is extended through this wire drain, so cancelled TCP work
is not quietly moved outside the denominator. Previously censored driver samples
remain censored. Run is single-use: no old history may be rebased onto a later
calendar. Default allocator still has tool task/queue/parser/history/oracle cost;
this is not zero-overhead instrumentation or unprofiled product CPU evidence.

Native real-transport 32/128 fixtures now create all client slots and synchronize
GET/PUT invocation with a barrier. HC2 accounting must show the actual connected
clients, values/retained entry bytes must match and explicit shutdown must release
server resources. HC1 claims logical client slots, not a verified physical socket
peak. Passing these tests does not establish native nonregression or saturation.

Scheduled RESP3/mTLS, multiple RESP connections, scheduled SET/multikey workloads,
representative error/miss/slow-reader load distributions and size transitions are
not implemented as numerical cohorts here. CPU/allocations/syscalls/copy counts,
retained/RSS idle-refill, quiet-host preregistration and feature-on hosted CI also
remain open. Both `pending` requirement flags stay true: instrumentation and a
small semantic grid are prerequisites, not the independently repeated admission
measurements. The existing 19-test `a6047895` check summary is historical and is
not overwritten to describe this extension.

## Retained extension verification

At clean source `a8440175478133dcea03ef4bdd7ef7114111b534`, all 29 tool tests
passed in each default/get-owner build: 12 scheduled, eight native and nine RESP
tests. The 52 performance-contract, 13 release-evidence, 23 governance and 16 doc
xtask tests passed. Scoped formatting, both strict check/clippy lanes, local
performance/doc checks, 17 governance checks, three documentation script tests,
link/sync checks and mdbook build also passed. The deterministic unresolved-wire
test uses virtual time: drain refuses a held response rather than issuing a
clean receipt, and explicit shutdown releases its owner.

The [extension check summary](local-runs/get-owner-resp-fifo-checks-a8440175.json)
retains exact source and Git blob identities and the fixture diagnostic. This is
functional evidence, not numerical nonregression, hosted CI or full workspace
verification. No benchmark series, B0 retry, host mutation or qualification ran;
the qualification manifest digest and historical summaries remain unchanged.

## Multiple RESP FIFOs and fixed-value SET extension

The single-connection GET baseline at `a8440175` and its check summary remain
historical. `start` retains that behavior; `start_connections` explicitly selects
1/8/32/128 accepted loopback TCP sockets and GET or SET of the preloaded fixed
value. All sockets in one control use the same production server and independent
fixture store; separate surface controls still do not share stores. Preload uses
one real socket, and setup/final GET/miss oracles verify visibility through every
socket. A 1 MiB SET control checks exact `+OK` and subsequent retained bytes.

Every original sequence routes to `sequence % physical_connections`. The route is
not reassigned according to response time or idle sockets. Records now include
`connection_id`: ordinals and response monotonicity are validated independently
on each socket, and changed routes/out-of-range connections are rejected. Equal
ordinals on distinct sockets are legal. There is deliberately no global FIFO or
total order across sockets. GET and fixed-value SET preserve the dataset digest;
they do not exercise conflicting writes, mixed command order or multi-key
atomicity. The wire samples remain one per command, never batch averages.

Each socket retains the existing finite queue/owner/parser/history limits. One
shared five-second wire-drain deadline covers the entire connection group, not
five seconds multiplied by socket count. A virtual-time fixture cancels a caller
on one connection, proves the other can complete with its own ordinal zero,
checks the exact group deadline, then releases the late response and joins both
actors. Construction owns all tasks before preload, so partial setup errors
abort owned tasks. Explicit shutdown visits all actors/server tasks even if an
earlier join fails; Drop abort is still only a fallback, not a clean receipt.
A connection-failure fixture aborts only one owned server task, confirms seven
other sockets still complete, and checks that even a failing shutdown joins all
client/server tasks without inventing a response timestamp for the failed socket.
If closure precedes actor admission, the driver still counts the operation error
but no FIFO record exists; the fixture must not require or fabricate one. An
initial assertion incorrectly assumed every transport error had a wire owner and
was corrected without changing outcome accounting or any numeric guard.

The functional matrix covers both operations, all four connection counts and
all three outstanding ceilings, plus invalid counts and 1 MiB SET. These small
fixtures assert protocol/ownership results, not a saturation knee, CPU/op,
allocation reduction or native nonregression. Product code, threshold/lock
identity, sealed B0/D3a artifacts and qualification manifest remain untouched.
Scheduled RESP3/mTLS, multi-key workloads, representative transitions and
memory/quiet-host numerical evidence remain separate open steps.

At clean source `83003e92bfe9652f430fd179931a781719679f79`, all 34 tool tests
passed per default/get-owner build (12 scheduled, eight native, 14 RESP).
All 105 focused xtask tests passed (53 performance-contract, 13 release-evidence,
23 governance, 16 doc), along with scoped formatting, both strict check/clippy
lanes, local performance/doc checks, 17 governance checks, three documentation
script tests, sync/link checks and mdbook build. The separate
[multiconnection check summary](local-runs/get-owner-resp-multiconnection-checks-83003e92.json)
retains source/blob identities, diagnostics and the semantic-only scope. No
numerical series or native floor is inferred; old summaries are not overwritten.

## Multi-key observer extension, not a batch product candidate

The next bounded adapter supports MGET, same-fixed-value MSET, EXISTS of live
positions (including duplicates), and DEL of an absent key. Batch sizes are
1/8/32/128. The production default is 128 entries; 256 is deliberately rejected,
not enabled by changing defaults. Real TCP oversized-MSET tests check the error
and every affected key afterward: the preexisting value survives and all new
positions remain absent. No batch product path or limit is changed.
Passing the expected-rejection oracle is a fixture result, not a scheduled good
success; scheduled controls never install an error-as-success expectation.

The reply parser accepts only flat arrays of bulk/null entries and signed i64
integers in addition to earlier scalar forms. It borrows the array body, caps
items at 128 and total encoded reply bytes at the unchanged 1,048,708-byte buffer
ceiling. A large individual value cannot multiply that ceiling by batch count.
Declared aggregate overflow, nesting, unsupported item types and integer overflow
fail loud; every prefix, null/empty/binary ordering and coalesced scalar suffix
are tested. Requests above 1,052,672 bytes are rejected before payload copying or
socket setup. Observer request construction, expected-value vectors, parsing and
validation still cost CPU/allocations; no zero-overhead/unprofiled claim follows.

One original offer still means one command, not one key. One complete MGET array
gets one timestamp and one histogram sample. Actual `response_items` validates
batch shape independently from the declared operation/count. Bounded protocol
error bytes are retained separately; errors cannot become scheduled success.
A cancelled partially received array keeps one FIFO owner until the final byte,
and its next integer reply cannot be reassigned or split into fake operations.

The scheduled semantic matrix has 48 cells: four operations at batches 1/8/32/128
on 1/8 sockets at depth 10, plus batch-8 depth-1/50 and 32/128-socket boundaries.
These are small functional checks, not the full frozen numerical workload grid.
Dataset bytes are unchanged after scheduled runs. Missing-key DEL is not a live
removal workload; separate TCP fixtures check deduplicated live DEL, duplicate
EXISTS, positional MGET with misses/empty/binary values, and MSET duplicate
last-write-wins. Concurrent MSET/MGET on different sockets accepts only the old
or new whole pair across 32 joined rounds. This bounded interleaving check is
not a general linearizability proof or distributed transaction claim.
A separate 64-command reference-model sequence logs seed 740074 and compares
actual TCP MGET/MSET/DEL/EXISTS with an independent map, including duplicate
positions, misses and empty/binary values. It exercises live mutations outside
the immutable scheduled cohort and is replayable, not numerical evidence.

RESP3/mTLS, expiration/quota/event/fault matrices, representative live-delete and
mixed workloads, matched native batch controls, allocator retention and repeated
quiet-host numerical evidence remain required. Historical GET/SET summaries and
B0/D3a packets retain their exact scopes; no product acceptance is reopened.

At clean source `35bbd4b894e4a39ba9d599d45b183bf4bcd17702`, 41 tool tests
passed in each default/get-owner build (12 scheduled, eight native, 21 RESP).
All 106 focused xtask tests passed (54 performance-contract, 13 release-evidence,
23 governance, 16 doc), with scoped formatting, both strict check/clippy lanes,
local performance/doc checks, 17 governance checks, three documentation-script
tests, sync/link checks and mdbook build. The separate
[multi-key check summary](local-runs/get-owner-resp-multikey-checks-35bbd4b8.json)
binds the clean source and blob identities to these semantic checks, not numerical
performance or native nonregression. Product/default/lock/qualification identities
and all older summaries are unchanged; no full workspace or hosted check is claimed.

## Independent native batches expose, rather than erase, semantic differences

The local adapter now issues BatchGet and same-fixed-value BatchPut through
three independent controls: direct ClientSurfaceState, HC1 HTTP and HC2 gRPC/mTLS.
The direct control has no socket, listener, certificate or daemon; its validated
identity invokes the existing `dispatch_verified_request`. It owns a separate
state/dataset just like each network control. Its tool-only slot mutex, envelope
construction, canonical hex-key mapping, expected-value vectors and byte oracles
remain part of the observed operation; this is not a new verified-session fast path.

The same seed, binary key/value corpus and digest remain shared across surface
fixtures. Supported batch sizes are 1/8/32/128. The local 1 MiB logical batch
budget includes repeated value bytes and 32-byte canonical hex keys, with checked
preflight before state/listener/PKI setup and payload-vector construction. That
bound does not replace the product's separate frame/value/batch limits. A maximum
single value still belongs to the earlier single-key fixture; batch multiplication
cannot silently create a many-MiB observer response. No product defaults change.

One batch is one original offer and one terminal observation, including the
entire ordered result vector. Count, indices/item ids, exact values, null versus
empty, errors and mutation-applied flags are checked before success. The 48-cell
scheduled grid covers three surfaces, two operations, four batch sizes and 1/8
client slots. Another 12 barrier-synchronized cells cover 32/128 slots at batch 8.
These are small semantic fixtures, not saturation, CPU/op or allocation series.

Source inspection and executable dispatch counts establish an important mismatch:
HC1 and direct BatchGet/BatchPut enter the client surface once; HC2's existing
`dispatch_invocation_inner` maps each batch item to a separate single-key request.
Its batch is ordered item execution, not atomic surface BatchPut/MSET. The SDK's
default batch cap is 1024: a 256-item GET succeeds with 256 dispatches, while 1025
items fail before dispatch. HC1/direct 256-entry BatchPut is rejected with no
partial mutation, and every affected key is checked afterward. An HC2 mixed
PUT/unapplied-CAS fixture explicitly retains the first write. Neither fixture
changes production limits or installs rejection/unapplied results as scheduled
success. No distributed transaction or general linearizability claim follows.

Consequently no pooled native metric or cross-surface MSET-equivalence claim is
admitted. Direct/HC1 atomic batches may later be compared with equivalent RESP
traces; HC2 needs its own ordered-item baseline and cannot stand in for that
atomicity control. A product HC2 batch-engine change would be a separate D1/D2
proposal, not an instrumentation shortcut. Embedded controls, live DELETE/EXISTS,
expiration/quota/event/fault matrices, matched RESP3/mTLS, retention/RSS and repeated
quiet-host measurements remain open. B0 is not retried; full D3/C74 stays closed.

Clean-source verification at `e1b981c094b9040ad902694e2fdded963964335a` passed
49 tool tests per default/get-owner variant (12 scheduled, 16 native, 21 RESP),
107 focused xtask tests (55 performance-contract, 13 release-evidence, 23 governance,
16 doc), strict check/clippy in both package lanes, scoped formatting, local
performance/doc gates, 17 governance checks, three docs-script tests, sync/link
checks and mdbook build. The separate
[native batch check summary](local-runs/get-owner-native-batch-checks-e1b981c0.json)
retains exact source/blob identities and the dispatch/atomicity distinction.
No numerical series, native nonregression, full workspace or hosted receipt is
inferred. Product/default/lock/qualification identities and older evidence stay unchanged.

## Plaintext RESP3: prove negotiation before adding a security cohort

The next adapter extension is dialect-explicit and remains plaintext loopback.
Existing `start`/`start_connections` still select RESP2; the new
`start_connections_dialect` performs HELLO 3 on every RESP3 socket before actor
creation, preload or the original-offer origin. Failed or timed-out setup drops
the socket/control owners rather than silently retrying or falling back. HELLO
is setup cost and does not become a measured command, response or good success.

The setup-only metadata reader caps its buffer at 4 KiB, accepts exactly seven
required unique fields and their fixed scalar/empty-modules shapes, and retains
only a bounded printable version string (64 bytes maximum). Negotiation has a
five-second safety timeout, tested with virtual time and socket closure rather
than a wall-clock performance assertion. Server/protocol/id/mode/role values are checked;
the version is diagnostic metadata, not independently verified source/binary
identity. Unknown or duplicate fields and unexpected types fail loudly. There
is no recursive map decoder in the scheduled actor. The first real test exposed
an invalid observer assumption: the production encoder uses a frame map whose
field order is not fixed. The oracle was corrected to check the key set and
values independently of order; no product behavior or threshold changed.

The reply parser now receives an explicit connection dialect. RESP3 null is
`_\r\n`, including positions in a flat bulk/null MGET array; RESP2 null remains
`$-1\r\n`. Each refuses the other's null. Null and empty bytes stay distinct,
all prefix splits and 128-null arrays are checked, and cancellation retains one
whole-array FIFO owner until its last byte arrives. Existing byte/header/item/
aggregate/history/drain limits are unchanged. Map/push/attribute/nested-array
responses remain outside the measured operation grammar and fail, not silently
become another scalar. Coalesced next-frame bytes keep their own owner.

Observation and per-socket wire records carry a dialect. The validator requires
every RESP3 connection's completed HELLO count and bounded version, rejects
mixed dialect metadata, and keeps RESP2's no-HELLO receipts separate. These
tool records are not a cryptographic TLS/source attestation. `GetMissing` is a
separate all-miss operation in either dialect, never mixed with hit-only GET.

The 56-cell RESP3 scheduled fixture uses seven operations (GET hit/miss, fixed
SET, batch-8 MGET/MSET/EXISTS/missing-DEL) at 1/8 sockets and depths 1/10/50,
plus 32/128 sockets at depth 10. Each cell has two offers per socket and one
sample per command; HELLO cannot multiply the denominator. Separate 1 MiB GET/
SET, ordered binary/null/empty/duplicate MGET and one-write HELLO3/miss/HELLO2/miss
fixtures check large payloads and immediate dialect transitions. The mixed
transition fixture is not a scheduled protocol-changing workload. RESP2's prior
grid additionally covers all-miss GET; old summaries retain their original scope.

This closes only a local plaintext RESP3 instrumentation prerequisite. It does
not close matched mTLS, full RESP3 command/subscription/attribute conformance,
expiration/quota/events/faults, representative mixed/live-delete traces, native
nonregression, allocator retained/RSS observations or repeated numerical pairs.
Product/default/lock/qualification and sealed B0/D3a packets remain unchanged.

Clean-source verification at `fc07bf382e6f0c481f381c81b4d1e87108f34db4`
passed 57 tool tests per default/get-owner variant (12 scheduled, 16 native,
29 RESP), 108 focused xtask tests (56 performance-contract, 13 release-evidence,
23 governance, 16 doc), strict check/clippy in both package lanes, scoped format,
local performance/doc gates, 17 governance checks, three docs-script tests,
sync/link checks and mdbook build. The separate
[RESP3 check summary](local-runs/get-owner-resp3-checks-fc07bf38.json) binds results
to source/blob identities and preserves both development diagnostics. It records
only semantic verification: no numerical series, native floor, full workspace
or hosted receipt is inferred.

## Security capability audit: TLS plus AUTH is not a matched mTLS cohort

The next prerequisite exposed a product capability gap at clean source
`2f6db5a4dedeeca423faef8c720d9543db76da4a`, before any secure adapter or numerical
series was added. `RedisTlsAcceptor::from_tls_config` reads the server certificate
and key; its PEM factory calls `with_no_client_auth()` and does not consume
`TlsConfig.ca_path` as client trust. The production HC/2 factory instead reads
that CA and configures `client_ca_root`. Both can encrypt traffic, but only the
latter requires the peer's client certificate at this listener boundary.

Existing production lifecycle fixtures connect to rediss with a client configured
with `with_no_client_auth()` and successfully execute AUTH/cache commands. The
negative server-CA fixture checks the client's trust of the server; it is not a
wrong-client-CA rejection. Plaintext and wrong AUTH are separately rejected.
RESP unit fixtures prove HELLO2/3 AUTH, NOAUTH before dispatch, per-connection
authentication and credential redaction. These are positive proofs of the
documented TLS/AUTH surface, not evidence of an undocumented mTLS surface.

`RedisRespServer::apply_auth` installs the listener's configured identity after
credentials match. An AUTH username is not a tenant selector and a CA-signed
certificate is not, by itself, an application authorization decision. HC/2's
foreign-client-CA/tenant fixture remains a separate control; its coverage cannot
be inherited by RESP. Neither a tool-only TLS wrapper nor a fabricated transport
label can supply a receipt for the production Redis acceptor.

The contract and enrolled guard retain this historical gap explicitly. After the
separate human authorization below, the guard checks the immutable audit receipt
and the new opt-in policy rather than requiring current production code to lack
a client-certificate verifier.
`matched_mtls_resp3_required` remains true; full D3, the native nonregression
floors, numerical comparison and release admission remain unchanged. A future
production TLS change must update this audit and its negative tests rather than
quietly leaving stale capability metadata. No product code/default/configuration,
lockfile, qualification manifest or sealed B0/D3a packet changes in this audit.

Adding opt-in required RESP mTLS is a separate product security proposal, not an
observer extension. It needs an approved trust-root configuration, compatibility
and rollback behavior, mandatory-versus-optional certificate policy, bounded
handshake/connection lifecycle and an explicit certificate/credential-to-listener
identity/tenant authorization rule. Real missing/foreign/expired/EKU/hostname and
authorization-denied cases must prove zero dispatch and complete cleanup. No
infrastructure run or new policy is authorized here. Until that decision, existing
TLS/AUTH controls may be developed under their own honest label and native
self-baselines remain available; the cross-security RESP/HC2 row stays absent,
not waived. Expiration/quota/fault/embedded/retention work remains separate.

Clean-source `b747502d4b63c4e0cdf1d1ee3857f439367f3545` passed the 12 selected
production TLS/AUTH/RESP fixtures, the HC/2 foreign-client-CA/tenant fixture in
default and get-owner variants, 109 focused xtask tests (57 performance-contract,
13 release-evidence, 23 governance, 16 doc), scoped format/check/strict clippy,
local performance/doc gates, 17 governance checks, three docs-script tests,
sync/link checks and mdbook build. The
[security audit receipt](local-runs/get-owner-security-audit-b747502d.json)
retains exact product/guard blob identities and scope. The entire observer suite,
full workspace and hosted CI were not rerun or claimed in this audit.

## Approved opt-in RESP mTLS prerequisite (2026-10-07)

The user explicitly approved the product security extension after the capability
audit. This authorization is separate from a performance candidate or release
admission. The historical audit and its receipts remain unchanged.

`redis_api.mtls_client_ca_path: Option<PathBuf>` is the sole opt-in switch. `None`
preserves existing plaintext/server-auth TLS behavior; a configured nonempty path
requires an enabled Redis listener, rediss, server TLS and required Redis AUTH.
The daemon rejects incompatible/dormant configuration. No global `tls.ca_path`
inheritance, optional-certificate mode or silent downgrade is permitted. The
operator supplies a dedicated inbound client trust bundle, at most 256 KiB and
16 certificates; malformed/empty/unreadable material fails before serving.

The standard rustls WebPKI client verifier must require a valid client certificate
with client-auth usage. A trusted certificate is only the transport gate: AUTH
still installs the listener's existing identity/tenant, and neither certificate
subject nor AUTH username chooses another tenant. This is not dynamic certificate
identity mapping or a new multi-tenant ACL system. Server-name/trust validation
also remains mandatory at the client. No certificate/key/password enters metrics
or retained evidence.

Only opt-in mTLS connections gain a five-second handshake safety deadline and a
128-connection owned task group. Completed tasks are reaped, overload is closed
before protocol dispatch, and listener shutdown cancels/joins remaining owners
so runtime accounting returns to zero. These are preregistered resource bounds
for a new security mode, not benchmark acceptance thresholds; legacy task/socket
behavior is not changed. No numerical series is allowed by this prerequisite.

Required local proofs cover defaults/old TOML, invalid configuration and bounded
CA startup, valid RESP2/3 plus AUTH, absent/foreign/expired/not-yet-valid/wrong-EKU
client certificates, wrong server CA/hostname, NOAUTH/WRONGPASS, connection-local
AUTH, same-state listener tenant isolation, handshake timeout, capacity rejection
and shutdown/disconnect resource cleanup. TLS 1.3 client-side handshake completion
alone is not proof of server acceptance; negative cases must prove no RESP
success and zero client-surface dispatch. Secure scheduled adapters and matched
native numerical comparisons remain a later stage after these product proofs.

### Production implementation and local proof boundary

The production daemon now selects `RedisTlsAcceptor::from_tls_config_with_client_ca`
only when the new option is present. The existing factory still calls
`with_no_client_auth()`; plaintext and old server-auth TLS defaults are unchanged.
The new path constructs the standard required WebPKI client verifier, reads no
global CA as inbound trust, and uses the existing RESP protocol/dispatch path.
CA input is read through a byte-limited reader and certificate-count-limited
parser. The pre-existing rule requiring complete global TLS material is not
weakened; its `ca_path` remains separate from this explicit inbound trust.

The serialized option is additive and omitted when unset. Rust consumers that
construct an exhaustive `RedisApiConfig` literal must add `mtls_client_ca_path:
None`; consumers using `..Default::default()` need no change. This is not an
old-binary enforcement guarantee for the new security mode.

The mTLS accept loop owns a bounded JoinSet rather than detached per-connection
tasks. Handshake failure, timeout, disconnect and completed owners release the
runtime guard; capacity refusal closes before dispatch. Shutdown aborts and joins
all owners, including authenticated sessions with incomplete RESP frames. A peer
may observe EOF, TLS truncation or TCP reset after this forced close; no partial
command is acknowledged as successfully dispatched. This is not a promised
graceful TLS close-notify or indefinite application drain.

`crates/hydracache-server/tests/redis_mtls_074.rs` uses generated local PKI and the
actual daemon factory/accept loop, not a tool-only TLS wrapper. Its tests cover
RESP2/3 HELLO AUTH, binary GET/SET and misses, absent/foreign/expired/future/EKU
certificates, wrong server CA/hostname, credential denial/redaction and
connection-local AUTH. Two explicitly configured listeners share one backend
to prove listener tenant separation; this does not create dynamic tenant routing.
Virtual time proves the five-second timeout, and real sockets prove 128-owner
capacity, rejection of a 129th socket, freed-slot reuse and shutdown accounting.
The disconnect fixture uses a 4 KiB binary payload, not a claim that the default
1 MiB request-frame limit accepts a full 1 MiB value plus framing.

Tests were added before implementation: compilation first failed for the missing
config field/error. The first PKI fixture omitted the existing global CA path and
failed `IncompleteTlsMaterial`; fixing the fixture did not relax production
validation. The forced-shutdown oracle initially accepted only TLS truncation,
not the observed TCP reset; it now accepts explicit close/reset outcomes while
still requiring empty RESP output and zero dispatch. Strict lint also required
a test struct initializer correction. These diagnostics are not suppressed or
interpreted as candidate performance data.

Rollback must block this listener before using an older binary that cannot
enforce the new option; removing the client CA switch is a policy downgrade,
not an automatic recovery. No certificate subject, secret or raw TLS error is
retained in evidence. The security extension remains opt-in and locally proven
only. Trust material is loaded at acceptor creation; hot CA rotation and CRL/OCSP
revocation are not implemented or claimed by this extension. Secure scheduled
observer support, equally secured RESP/HC2 cohorts,
expiration/quota/fault/retention controls, hosted CI and release qualification are
still separate, unfinished work. No numerical series, B0 retry, host service
operation or expensive infrastructure run is authorized by these checks.

### Clean-source verification receipt

At `8565dbfd11ae1ce6d8f20146a37058b8bfb77728`, 87 server unit tests, 28
lifecycle tests, three configuration property tests and 11 mTLS integration tests
passed, with the latter 11 repeated successfully with experimental get-owner.
The observer suite passed 57 tests in each default/get-owner variant, serialized
and again with normal test parallelism after builds completed. The focused xtask
suites passed 58 performance-contract, 13 evidence, 23 governance and 16 doc
tests. Scoped all-target check, all-feature strict clippy, formatting, local
performance/doc gates, 17 governance checks, three docs-script tests, sync/links,
mdbook and diff checks passed. Source was clean before and after these checks.
The [separate local summary](local-runs/resp-mtls-product-checks-8565dbfd.json)
binds those results to exact source blobs; it is not a release-admission receipt.

The first pinned observer repeat ran alongside builds and failed two plaintext
preload tests (`multiple_resp_connections_keep_local_fifo_and_fixed_set_oracles`
and `resp3_negotiates_every_socket_and_keeps_command_denominators`) with
`RESP2 transport preload failed`. Their cause remains unattributed. Both repeated
successfully on the same source, serialized and then normally parallel, without
changing operation/concurrency grids, timeouts, seeds, workload or thresholds.
The failed attempt is retained; neither its long execution nor the clean repeats
are a throughput/CPU/latency claim or proof of robustness under external pressure.

Reproduce the focused product and observer checks locally:

```powershell
cargo test -p hydracache-server --lib --test redis_mtls_074 --test server_lifecycle --test config_properties --locked -- --test-threads=1
cargo test -p hydracache-server --test redis_mtls_074 --features hydracache-redis-compat/experimental-resp-get-owner-074 --locked -- --test-threads=1
cargo test --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --locked -- --test-threads=1
cargo test --manifest-path tools/get-owner-scheduled-controls-074/Cargo.toml --features get-owner --locked -- --test-threads=1
cargo test -p xtask --test performance_contract_074 --test release_evidence --test release_governance --test doc_check --locked
cargo check -p hydracache-server -p xtask --all-targets --locked
cargo clippy -p hydracache-server -p xtask --all-targets --all-features --locked -- -D warnings
cargo xtask performance-contract-check --release 0.74
cargo xtask doc-check
cargo xtask release-governance-check --release 0.74
```

The observer's isolated normal-parallel repeats use the same two `cargo test`
commands with `-- --test-threads=1` omitted. No command above executes numerical
release qualification or operates a rented host.
