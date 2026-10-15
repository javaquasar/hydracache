# Original kernel refusal attribution

Baseline `914aa749369a78a57ae8aebb8e95c8a878366fbf`. The original controller
fixture once refused Input(Kernel(Invalid)) during build binding before any
attempt write; isolated and complete same-source reruns passed. Neither those
successes nor the timesync journal identify the refusing predicate. Preserve
the historical packet and leave its cause unresolved until an actual observation
supports attribution.

Add private cfg(test)-only, scoped, thread-local first-refusal capture around
the original controller fixture. Record only a static source site, file/line,
fixture phase and six bounded numeric clock facts for ordering/hold errors.
Never retain a packet, token, credentials, key or raw proc/config contents.
Panic reporting must preserve the original panic and restore the prior scope;
nested and concurrent fixtures cannot share or overwrite first refusal.

Instrument Invalid construction sites in kernel admission and origin, the paths
reachable during original revalidation. Ancillary parsing remains unchanged.
A macro expands to the same KernelError::Invalid in non-test builds; the capture
module and numeric expressions exist only under cfg(test). A separate Option
macro keeps the original eager ok_or(Invalid) in non-test builds, using lazy
recording only in tests so successful checks never manufacture trace events.
Keep every predicate, observation order, refusal
latch, budget and 60-second hold unchanged. No retry or clock fallback is added.

An independent token-level source comparison against baseline must erase only
the exact approved trace header, replace trace invocations with their original
enum constructor and expand the Option macro to its original ok_or. Normalize
only non-semantic question-mark joint spacing caused by line wrapping. Everything
else must match. Canary mutations to a guard or deadline must still differ.
This proves restricted source equivalence, not binary identity or performance.

Capture the existing kernel baseline and a missing-capture-API red test first.
Cover healthy no-event behavior, eager-constructor false positives, first-event
retention, nested/unwind restoration, thread isolation, deterministic clock/
hold/projection/namespace refusals and redaction. Use seed 0x0740c096 for the
classification matrix. Repeat the original-input group at most three times on
current Rust and once on MSRV, without concurrent heavy builds. Preserve every
failure. A newly observed refusal may identify only its own occurrence; it does
not retroactively establish the earlier failure's cause.

Run kernel and original-controller focused regression, the joined manual crash
matrix, current/MSRV checks and strict Clippy, source-equivalence/static gates,
format, registry, docs and closed local admission checks. Retain a separate
packet and update the article/checklist with established findings versus open
attribution. Commit instrumentation separately from evidence. No server,
GitHub, actual signing credentials, product workload, qualification, push,
production enrollment, original-start or writer-revocation rights are opened.

The first strict Linux Clippy run correctly rejects globally lazy construction
once cfg(test) recording disappears. Keep that negative. The compile-time Option
split above resolves it without lint suppression or moving any observation.
