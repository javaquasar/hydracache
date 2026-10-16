# Worker proof reuse second screen

This is a new W11 local tooling experiment against 73f71f07, not a continuation
or retry of the [first stopped screen](local-runs/worker-proof-reuse-20261016/README.md).
That candidate and its Setup=1 failure remain withdrawn and immutable. The one
runtime hypothesis and practical floor are unchanged: reuse authenticated original
worker-document proof only inside the same checked owner, preserving live checks.
No product workload, server, qualification, Starting or push is admitted.

## Construction site accounting fixed before observation

The unchanged controller_worker_fixture calls Model::authority, which invokes
verify_worker_policy to initialize modeled authority. with_progress_original then
calls verify_kernel_policy, whose verify_policy creates a distinct checked owner.
Both full constructor verifications are required even for identical documents;
there is no global cache or authority sharing between these owners. The signed
document verifier has no clock-validity predicate. All enclosing live source,
epoch, peer/kernel, OIDC, packet, challenge and deadline checks remain unchanged.

Before implementation add a second portable mechanism test with two independent
constructors and eight revalidations per owner: old full verification count 18,
candidate count exactly 2. Keep the prior single-owner mechanism test (10 to 1),
exact old private cfg(test) oracle, 128 seeded error/latch comparisons, sixteen
projection-field mutations and four concurrent independent owners. Preserve both
mechanism reds. Neither test may merge two constructions into one verification.

The real original-fixture stage expectation is now fixed as Setup=2 and
Fence/Retain/Observe/Publish/Bind=0. This is constructor accounting, not a relaxed
performance floor. Baseline full-worker counts remain 99/72/908/308/2696/652;
request-authentication and request-signature counts remain 11/8/96/32/272/66 on
both variants. Change only the old implementation-specific worker-count assertion;
every action, semantic/file/lock assertion and measured stage boundary stays exact.

## One runtime substitution

Reuse the same previously reviewed substitution and no second optimization:
retain prior-refusal, exact original trust/key/body pin/epoch, context, bound and
envelope SHA256 checks; replace only the repeated full verifier/projection equality
with canonical-LF retained-projection SHA256 against the original body pin. The
constructor, fields, retained buffers, lifetime, public API and error order stay
unchanged. This relies on the same SHA256 integrity assumptions as existing pins.
Keep exact baseline restoration and negative source canaries in static contracts.
Registry command, manual tier and 900-second harness budget stay unchanged; only
its descriptive reason may change as in the withdrawn candidate. No old test is
skipped or serialized, and the 60/10-second signed/progress windows do not change.

## Independent observations and fixed decision rule

Use the retained baseline libtest from 78d8d9fc, SHA256
7e8247610919a8dc39f3f6b7f6644944084a36a78cf949045fe9a0eaa7a946ad.
Selected runtime, fixture and instrumentation bytes at 73f71f07 must equal that
baseline before mutation. Verify its hash and run one separate baseline preflight;
do not import any old stage observations into this screen. Freshly build and hash
the new candidate. Hash both again after the screen; run no concurrent build or
whole-workspace formatting during real-clock regression or paired measurements.
These receipts establish local consistency, not trusted source-to-binary attestation.

After focused semantic/reference gates, run five independently started pairs in
exact B/C, C/B, B/C, C/B, B/C order, with the same binary command, cwd and exact
registered manual probe. Do not retry, discard or replace any failed invocation.
On failure stop the screen, retain every capture and revert only this isolated
candidate. Do not append samples to the first experiment.

For Publish and Bind estimate exp(mean(log(candidate_wall_ns/baseline_wall_ns))).
Use 10000 paired percentile-bootstrap resamples, seed 0x0740c0a0, xorshift32
(left 13, unsigned right 17, left 5), modulo-five sampling. Reset the same seed
per stage. Quantiles linearly interpolate at (10000-1)*p for p=0.025/0.975.
All five pairs are included with no warmup exclusion. Both upper 95% bounds must
be <=0.80. A complete but ineffective screen also requires an isolated rollback.
Signature/authentication counts must remain exact and both constructors survive.
No result is production CPU/op, native/RESP goodput or allocation qualification.

## Verification and still open work

Run Linux/Windows current 1.94/MSRV 1.88 affected-package all-target/all-feature
check and strict Clippy, all five portable proof tests, current/MSRV original-input,
kernel, context/projection/IPC/file/policy, credentials, causal manual four-case
proof and 28 compile-fail examples. Preserve any IPC negative without retry; run
only targets Cargo did not reach afterward, clearly separately named. A separate
IPC failure is not removed by a green timing screen or MSRV result.
Run static contracts, formatting, documentation/registry/local performance checks;
ship admission stays closed. Full release verify and new SIGKILL boundaries belong
to later milestones. Historical Time/Kernel(Invalid)/Host(Busy) causes, shared-load
robustness, actual enrollment, authenticated start, all-writer cessation, remaining
cells and host rehearsal remain open unless separately proved.
