# Observed initial reservation evidence

The [preregistration](../../diagnostic-observed-reservation-plan-20261016.md)
starts at `0389978f1ff5d7eeacb99c497f9ef8cbfd6be08a`. Runtime is
`77fdb7b8`; final source is `82312a02fc9ebc9a45360a7fb04d577c31d58468`,
tree `85d404034f45ceef9ad3dde5c6c12188cff7e5d8`.
The [design](../../diagnostic-original-collector-design.md#observed-initial-state-under-the-same-owner)
and [contract](../../diagnostic-observed-reservation-contract.toml) distinguish
in-memory observations from durable lifecycle acknowledgement.

## Confirmed behavior

The acknowledged original request is consumed into one observed reservation
owner, retaining its original controller, kernel/worker/authority and fence.
First successful observation mints revision-one, cell-zero Reserved from the
original request identity and receiver boot/CLOCK_BOOTTIME. All three initial
clock fields share one sample. Later observations check clock order without
moving state or extending the original receive hold or signed request window.

The exported state for signing is a cloned data value. It cannot replace the
private state. The distinct collector guard accepts only signed bytes and binds
them against the frozen state through the existing original verifier. There is
no old-guard conversion, replacement timestamp/peer/state parameter, second
journal opener or released/reacquired fence. Failure is sticky across the same
original owners, and subsequent refused calls return before new I/O.

Six new runtime tests bracket the first origin using real local CLOCK_BOOTTIME,
check repeated exports and clone mutation, unchanged retained request bytes and
metadata, held fence and post-drop context refusal. Seed 0x0740c099 shuffles six
independently constructed, validly signed changed state digests: three initial
clock fields, lease identity, revision and completed cells. They all refuse
without changing the original retained request. Separate cases cover prior
refusal, missing records, late worker/OIDC/inventory drift and independent owners.

Trust documents, keys and nonroot temporary storage roots are synthetic fixtures;
kernel message, sender and clock observations are actual local Linux inputs.
The fixture runtime never performs the mutations used by refusal tests: only
test code changes or removes its own files, and restoration cannot heal refusal.
Positive and signed-mismatch cases separately preserve original stored bytes.

## Development observations and test boundaries

Baseline passes eight existing original-collector tests. The first compiler red
also exposed a missing MetadataExt test import; its tool-truncated diagnostic
capture is retained as red.log, not presented as an isolated missing-API proof.
After fixing that import, red-clean.log fails only on the absent
into_observed_reservation API. The first implementation passes all six new tests.

The first static suite passes 171 tests and fails the new gate because a literal
source marker did not accept rustfmt's multiline clone expression. The corrected
check ignores whitespace for that expression while preserving the same required
operation. The focused gate and subsequent 172-test suite pass. This is a test
matcher correction, not a runtime failure fix, changed admission predicate or
relaxed timing budget. Historical kernel guard/token canaries remain intact.

Final-source regressions pass on both Rust 1.94 and Rust 1.88: 82 original-input
tests (one existing manual case ignored), 30 signed-projection tests, 26 kernel
tests (one existing manual case ignored), and ten compile-fail examples. Neither
original-input log contains an unexpected first-refusal event. All eight final
Linux/Windows current/MSRV check/strict-Clippy lanes pass with all targets,
all features and locked offline dependencies. The registry remains unchanged
at 176 discovered/registered gated tests. Local non-promotable performance
contracts pass; require-ship refuses the unchanged unresolved candidate and
qualification boundary as expected.

Both original supplied-state and new observed-state APIs remain read-only with
respect to lifecycle publication. The existing request writer process-loss run
is regression of the unchanged storage engine, not a SIGKILL proof for a new
Reserved/Starting write or root constructor. Intentional caught panic output
from kernel/classifier canaries is not a reproduced original Kernel(Invalid).
That historical refusal remains unexplained.

The unchanged writer regression passes nine actual owned SIGKILL cases and one
uninterrupted control, with twenty fresh read-only inspections. Owned helper
death, lock release and retained context are checked. This establishes local
process-loss regression, not power-loss/reboot durability. Workspace formatting
and documentation checks pass. The manifest retains 31 command captures and
33 hashed artifacts, including the compiler/static negatives above.

## Remaining publication and execution

Observed state is not durable Reserved/Starting acknowledgement. A new consuming
publication must preserve this owner and original request object, introduce an
explicit versioned record and exact inventory, and pass its own write-boundary
SIGKILL/lost-ack matrix. Existing one-document attempt checks and legacy
regular-file intent transfer cannot be used as a generic adoption route.
The signing round trip must fit the unchanged original request/packet lifetime.

Root ancestry/output creation, authentic executable/streams/process/manager/
cgroup start, autonomous watchdog and all-writer cessation remain open, as do
production trust enrollment, later cells and host rehearsal. This packet changes
no product, signed/durable format, compatibility register, frozen qualification
input or threshold. No server/workflow/product workload, six-hour/24-hour stage,
full workspace release verification or push is included.

Run ./verify.ps1 for source-object, unchanged-input, log/count and byte-hash
consistency. Logs are normalized UTF-8/LF; empty command output has one final LF.
Caches/dependencies are reused. Commit pins identify final source bytes, not a
clean-checkout build, reproducible binary or signed execution attestation.
No numerical performance result or production readiness follows.
