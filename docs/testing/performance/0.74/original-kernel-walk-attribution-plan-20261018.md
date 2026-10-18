# Original kernel context attribution

Baseline is `be06c237c3c8c3b6cf4e5eb608c90d0f9feaa47c`. The retained worker
control localizes about 69 percent of its instrumented wall span to two kernel
context checks. Attribute their actual nested calls before selecting a change.
Keep all 46 effect-local frame boundaries, live predicates, original call order
and 300/60/10-second origins/deadlines unchanged. No mutable observation cache,
validator substitution, deadline renewal or production optimization is proposed.

Add only cfg(test) entry spans for ten existing functions: packet context,
sender validation, sender liveness, sender projection parsing, origin validation,
boot document, named namespace, clock sampling, worker-origin join and bounded
proc document reading. Retain the previous six worker spans. A separate numeric
thread-local tracer records counts, inclusive wall nanoseconds, time outside
instrumented direct children and parent/child counts/times. Removing the ten
exact insertions and two registrations must restore every old Rust file.
The tracer records no document, policy, credential, path or timestamp payload.

Use three fresh original fixtures per compiler and 32 consecutive worker
validations per fixture, before reservation/challenge. Require full original
OIDC validation before and after, unchanged request and worker/OIDC bytes, zero
IPC authentications/signatures, no refusal and unchanged six-worker counters
`[32,64,128,128,64,32]`. In the ten-kind order above, preregister counters
`[64,128,256,256,512,1024,2048,64,64,1536]` per batch.

Preregister thirteen direct edges per batch: packet→sender 128,
packet→worker-origin 64, packet→sample 64; sender→origin 256,
sender→alive 256, sender→projection 256, sender→document 256;
origin→boot 1024, origin→namespace 2048; alive→document 256;
boot→document 1024; sample→origin 128; worker-origin→origin 128.
Only packet context is a root, with 64 calls. Require exact edge membership,
node incoming counts and parent inclusive-minus-outside equality to its direct
edge times. Preserve 60 node, 78 edge and 36 enclosing-worker rows separately.

These are instrumented wall controls, not exclusive CPU/IO/allocator costs.
Bounded-reader timing includes buffer creation and pread; it cannot establish
an allocation owner or allocation count. Do not sum nested node spans, equate
sampling's inclusive time to clock syscall cost, extrapolate to the full
preparation bucket or claim a speedup. All observations occur before the
ten-second challenge. Unit controls cover disabled/empty capture, nested edges,
recursive kinds, nested capture/unwind restoration, thread isolation and an
unfinished escaping span refusing capture success.

Build normal GNU release Rust 1.94 and MSRV 1.88. Run one exact ignored host
invocation per compiler; stop on the first unexpected failure without retry.
Use only authorized nonroot correctness fixtures, a fresh owned 0700 runner,
exact image/script hashes, unchanged boot/supervisor/installed receipts and
independent executable-object postflight. Existing helpers also use separate
private TempDir roots, including a direct-/tmp authority leaf. Do not redirect
TMPDIR or claim single-parent filesystem containment.

No release installation, service/clock changes, root fixture child, manager
send/start, product workload or measurement qualification is allowed. Preserve
old evidence. Run current/MSRV focused tests, affected all-target checks and
strict Clippy, exact-source contracts, formatting, registry, documentation and
local non-promotable performance contracts. Bind source, literal embedded
inputs, compiler artifacts, raw transcripts and derived rows in a closed packet.

## Result

Source `da6637907b377597ae37675f7f6e5e125b553ac0` passes both exact host
invocations on first execution: six fresh fixtures and 192 worker validations.
All 174 node/edge/enclosing rows, counts and disjoint wall partitions verify in
the [evidence](local-runs/original-kernel-walk-20261018/README.md). Origin's
inclusive share is 82.2–82.5 percent of kernel context; clock sampling mostly
contains origin checks. The large bounded buffer for the fixed boot document
is a next allocation hypothesis, not an accepted win. Live checks, effect
boundaries, deadlines, supervisor/install and execution admission are unchanged.
