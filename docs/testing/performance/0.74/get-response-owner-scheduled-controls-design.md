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
