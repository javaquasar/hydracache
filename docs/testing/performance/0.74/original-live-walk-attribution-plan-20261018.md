# Original live worker walk attribution

Baseline is `b2adb5b73a2f66fc690637260c100b5c8489aa35`. The preparation
packet at that source accounts for 46 effect-local eight-step passes in joined
outputs, with 782 IPC authentications. Their inclusive authentication spans do
not explain the approximately 7.2 second join. Preserve all 46 boundaries.

Measure the actual retained worker validator before reservation, not a copied
validator or a replacement for ReservedLive. Add six test-only numeric RAII
spans at the entry to the existing kernel-worker, worker-policy, worker-source,
active-authority, kernel-context and local-context functions. Removing those
exact cfg(test) insertions and registrations must restore every existing Rust
source byte-for-byte after LF normalization. Normal builds have no new code.

Three fresh original fixtures per compiler each perform 32 consecutive worker
revalidations inside a capture, with full original OIDC validation before and
after. For each batch require 32 kernel-worker, 64 worker-policy, 128 worker-source,
128 active-authority, 64 kernel-context and 32 local-context spans; IPC
authentication and request-signature counts must both be zero. Require unchanged
request and worker/OIDC bytes, no refusal and no reservation or output creation.
Use normal GNU release Rust 1.94 and MSRV 1.88, one exact ignored invocation
per compiler, with no retry after an unexpected failure.

Retain numeric call counts, inclusive wall spans and time outside instrumented
direct children for each kind. These are instrumented wall-time controls, not
CPU, syscall, lock-contention or exclusive filesystem ownership measurements.
Do not sum nested inclusive spans. The controls run before the ten-second
progress challenge; they neither measure nor enlarge that critical-path window.
Unit controls must cover empty, nested, nested-capture, panic restoration and
thread-local behavior. No policy/token/path data is emitted by the tracer.

Run only authorized nonroot server fixtures with exact image hashes, stable
boot/supervisor/installed-artifact guards and independent executable postflight.
Use a fresh owned 0700 runner directory. Existing helpers also allocate private
TempDir roots, including the strict direct-/tmp authority leaf; this is not a
single-parent filesystem containment claim. Do not redirect TMPDIR or change
that rule. No root fixture child, install, service/clock change, manager send,
product workload or measurement admission is allowed.

Run focused current/MSRV tests, affected all-target checks and strict Clippy,
source contracts, formatting, registry, documentation and local non-promotable
performance-contract checks before committing the selected source. Retain the
first failure if any. Report all 36 span rows with source/image identity and
unchanged live predicates, call order and original deadlines. Select any later
optimization separately; these observations alone do not authorize caching or
removing a live walk.

## Result

Selected test-only source `bd65514b144c6b3575f0255a2dc3f10fb23cdbc3` passes
all six fresh current/MSRV host fixtures on first execution: 192 validations,
36 span rows and unchanged original inputs. The
[evidence](local-runs/original-live-walk-20261018/README.md) verifies exact
parent/child accounting and preserves initial static-guard failures separately.
Kernel context contributes about 69 percent of the measured worker wall span,
active authority about 12 percent. This supports finer kernel-walk attribution,
not an optimization, CPU/IO ownership claim, deadline change or server admission.
