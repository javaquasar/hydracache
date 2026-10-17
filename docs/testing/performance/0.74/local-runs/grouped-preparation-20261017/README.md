# Grouped nonexecuting collector preparation

The test-only grouped candidate passes the first healthy screen, six repeated
healthy original chains and 28 persistent-drift/expiry cases across Rust 1.94
and MSRV 1.88 normal GNU release binaries. It replaces three separate
material/message/alarm brackets with one opening/closing bracket in this
candidate only. Production bodies, APIs, observations and deadlines remain
byte-exact. No message is sent and no process or guardian starts.

Baseline is 67419018; the [protocol](../../diagnostic-grouped-preparation-plan-20261017.md)
is preregistered in 1ec9d6be and implementation is 76e8a1b7. The user's request
to continue followed the explicit grouped-preparation proposal. The candidate
is deliberately private to cfg(test), not production adoption. Its six-file
source capsule, committed blobs and both binary digests are in this packet.

## What the candidate changes

The existing material_round performs the opening full original frame on the
acknowledged outputs. The candidate then retains the original signed projection
and Starting state, inspects the same signed nonexecuted fixture bundle, calls
the existing material binding, and constructs/inspects the unsent FD-bearing
message through the existing worker/message functions. The existing boundaries
and original KernelPacket time sample provide a provisional absolute BOOTTIME
alarm. This provisional timer cannot authorize any operation.

Closing calls the unchanged original_round. It independently repeats the full
frame, bundle/build, original state transition, time origin and message/writer
checks. Its bounds must match the provisional bounds; alarm inspection follows
before material becomes ready. No caller checkpoint runs after this closing
observation. Errors refuse the original chain and retain the owner/fence until
failure drop, including partial material/message/timer acquisition.

Four intermediate complete rounds are absent from the candidate schedule.
Persistent drift at closing must refuse, but a mutation restored between the
endpoints may escape observation. This is the accepted experimental tradeoff,
not equivalent continuous monitoring or a claim that fewer observations preserve
all temporal detection guarantees. The unchanged reference validators remain
available and are exercised after every healthy construction.

## Healthy results and timing scope

The first Rust 1.94 screen passes construction and complete unchanged reference
revalidation. Three further fresh samples per compiler also pass. Every grouped
construction makes exactly 34 IPC authentications/signatures; every full
reference revalidation also makes 34. The earlier three separately bracketed
preparation calls each made 34, but their overall healthy/reference gate failed.
This count contrast is not a paired latency improvement estimate.

| Repeated original fixture span | Rust 1.94 milliseconds | Rust 1.88 milliseconds |
| --- | --- | --- |
| Grouped preparation | 362.250–363.193 | 357.404–358.891 |
| Unchanged reference revalidation | 335.145–335.687 | 330.826–334.325 |

These inclusive spans exclude earlier output preparation, fixture setup and
teardown. The separate screen preparation is 362.640 milliseconds and its
reference check 337.919. `phases.json` retains all fourteen exact rows with mode,
compiler, sample and phase; the screen is not merged into repeated ranges.
No first-time refusal or refreshed timestamp occurs. All original bounds remain
300 seconds from Reserved, 60 from Starting and 10 from the original progress
packet's kernel receive time. A later Starting controller timestamp is not the
loss-deadline origin. Intent, Reserved and Starting record bytes stay unchanged.

## Persistent drift and expiry

Each compiler uses fourteen independent original chains. Trust bytes change
after each of opened/loaded/bound/message/armed. Other cases alter bundle
content or inode, write an output, corrupt the projection, Starting state,
writer flags or unsent message, disarm the timer, or wait out the original
progress ceiling without changing clocks. Binding-fault cases must identify
their expected error family, not pass because a later Time error masks them.
Expiry requires the original progress-age refusal with the fixed ten-second
bound. Further validation is sticky Refused, durable records do not change,
and the fence stays held until failure drop. Fresh context refuses retained
history after both successful and failed owner drop.

These are persistent mutation and real-kernel timer checks, not new SIGKILL,
lost-ack, independent-guardian or all-writer cessation proof. The group creates
no new durable record; actual interruption and delivery/ack boundaries need
their own declared process-loss gate before extending execution admission.

## Checks and provenance

Both compilers pass four release and four debug alarm/kernel/reference units,
six trace units, 41 targeted integration cases, five compile-fail examples,
all-target check and strict all-feature Clippy. Windows supervisor/xtask check
and strict Clippy pass. All 194 static contracts pass, including exact old
runtime bodies; the registry validates 183 discovered gates. The three new
nested libtests have explicit manual entries and are independently listed by
both release binaries; they are not three extra auto-discovered integration
targets. WSL workspace formatting, documentation and the non-promotable local
performance contract pass. Full milestone/require-ship qualification is not run.

The prior syscall packet passes its unchanged verifier in an archive of exact
source 67419018. The first archive-helper run detects active-index byte drift
after the historical verifier succeeds; its cause is not attributed and the log
is retained. The final helper selects an owned index copy, disables optional
index refresh and verifies the active index hash stays unchanged. It does not
rewrite the historical verifier or its frozen expected values.

## Server isolation and remaining work

All server writes stay nonroot under
/tmp/hc074-grouped-preparation-20261017.LmaZ0c, owner 1000:1000 mode 0700.
The signed bundle is copied as data and never executed. Before/after guards
and independent executable-object postflight preserve the original boot,
supervisor PID 8839/start/NRestarts 0, installed binary and receipt hashes,
absent campaign/context markers and absent diagnostic installation. No release
install, supervisor stop, root mutation, clock change, observer, product load
or expensive Actions occurs. Qualification manifest raw SHA-256 stays
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

Next is a real process-loss gate around grouped acquisition/return and explicit
ack-loss handling, followed by a reviewed production composition rather than
exposing this fixture API. Autonomous guardian lifetime, authenticated original
send/start, all-writer cessation, completion, later cells, enrollment/installation
and release qualification remain separate unfinished obligations.
