# Original preparation budget attribution

Baseline is `0847b53f37a0fd79ad64845e53dc38354d99e552`; its approved runtime
source is `2ed04dc8`. Twelve grouped-message fixture rounds leave a minimum
observed 25278110 ns before the original 10 s challenge bound. This diagnostic
will attribute the earlier path without changing validation frequency, guard
bodies, origins, clocks, deadlines, durable writes or execution admission.

Add one ignored test below the existing addressed-message fixture. Its
orchestration helper copies the existing accepted fixture exactly, with only
test-only numeric stage/landmark insertions and a diagnostic accumulator
argument. A source guard must remove those exact insertions and recover the
entire original helper. It is not a second validator. All existing Rust sources
must remain byte-equivalent apart from the exact cfg(test) registration and
new source-contract registration. No production instrumentation is added.

Use the existing validation_trace capture, without adding counters to runtime
guards. Eight outer buckets are prechallenge reservation, challenge creation,
response acceptance, consent preparation, consent acceptance, joined outputs,
enrollment and manager/probe bookkeeping. Two nested captures report manager
preparation and later validation with the existing test-only socket transfer.
Their inclusive wall spans must not be added to the enclosing bucket or to
authentication/signature durations. Authentication includes signature work;
Instant and BOOTTIME durations include scheduling and are not CPU residency.

Retain numeric BOOTTIME landmarks and the issued field read from immutable
canonical challenge data. Confirm it equals the original frame's actual issued
origin. Record challenge-ready, response-ready, consent-prepared, consent-ready,
outputs-ready, enrollment-ready and both manager round endpoints. Defer all
printing until after the measured critical path and owned cleanup. No packet,
token, key, principal or path is included in numeric reports.

Run three fresh healthy samples for GNU Rust 1.94 and 1.88 in a new nonroot
0700 temporary directory on the already authorized dedicated host. Retain exact
stable boot/PID/installed binary/receipt and no-campaign pre/post guards. Each
sample preserves original request semantics, full child FD inventory, journal
bytes, fence retention, fixed 300/60/10 s bounds and two 34-check manager rounds
with two context checks each. This is a distinct attribution cohort, not a
repeat or replacement of any previous acceptance or failed cohort. Stop at the
first unexpected failure, retain it and do not retry into success.

Record approved-image baseline, source equivalence/mutation guards, current/MSRV
focused tests, all-target checks and strict Clippy, portable controls, registry,
format, docs and local non-promotable performance contracts. Preserve selected
sources, complete build inputs, images and raw diagnostic output. Report every
compiler/sample/bucket separately. Do not infer filesystem/CPU cost by subtracting
overlapping spans, choose thresholds from the observations or claim a product
speedup. Further changes to observation boundaries require their own agreed
experiment. No installation, supervisor/clock operation, real systemd dispatch,
trust enrollment, product workload or expensive qualification is permitted.
