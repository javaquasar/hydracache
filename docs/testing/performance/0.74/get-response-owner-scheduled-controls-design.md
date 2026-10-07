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
