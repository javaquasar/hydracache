# Model stage custody safe spawn observations

The remaining spawn observation now has a separate model-tested prototype for
preparation, create-to-retention and membership/release. It adds no probe in the
successful-create-to-owner-publication gap and preserves the original failed-
create error path. The prototype is not connected to the native caller; this
stage establishes model compatibility, not a measured cost or release admission.

## Implementation and ownership boundary

Source `7a6556aa0f04f4e2cf0d716968e5eab36f2cc529` adds
`spawn_frames.py`, its tests, design/protocol and a compatibility entry. The
original CI API, existing caller/leaf recorder and Rust worker remain byte-
identical to the [first native leaf packet](capture-leaves-native-result-20261026.md).
There is no new CLI or hidden native entry point.

Preparation starts inside the original try and ends before create arguments
are evaluated. Create-to-retention starts before the unchanged CreateProcessW
condition and ends inside the owner setter, after the original `_child = value`.
It includes call arguments, creation, WindowsChild construction and publication;
it is not pure CreateProcessW cost. Identity and initial process checks retain
their existing leaves and order after publication. Membership/release surrounds
the original membership test, deadline test and conditional resume, ending
before the original return/finally.

The inherited-duplicate and attribute-list finally is unchanged and untraced.
No clock runs between successful CreateProcessW, WindowsChild construction and
owner publication. No probe precedes the original error read on failed creation.
No fallback launch, PID reopen, breakaway, new wait or cleanup mechanism is added.
The existing diagnostic Job policy remains separate from production authority.

Six probes add at most eighteen coordinator clock reads per spawn. With the
existing outer and leaf layers, the recorder maximum is 102 reads per worker,
excluding original metric/deadline observations. These are instrumentation
budgets, not measurements. CPU remains coordinator-wide; nested scopes are not
additive. Owner leaves, finally and intervening bookends prevent treating these
three spans as a complete partition of the original outer metric.

## Differential and fault controls

All fifteen new tests and 115 unchanged regressions pass: 130 combined. Fake ABI
calls exercise the actual Python spawn body with synthetic ctypes out parameters
and injected clocks; they create no process or native Job and read no live clock.

Restricted AST comparison removes only literal marks on `spawn_frames`, then
compares the complete API body and retained-owner setter to their originals.
Differential controls retain argv/cwd/flags, handles and inheritance, original
call order, one API deadline read, first refusal and cleanup. The no-standard-
handles branch, missing-program error, partial setup, identity/initial checks,
membership, resume and composed duplicate-close errors remain covered.

All eighteen added clock-read positions have ordinary and fatal cuts. Ordinary
errors freeze the recorder while original effects and cleanup continue. Fatal
cuts before creation produce no child; every cut after creation leaves the child
held by the original owner and accessible to unchanged cleanup even when the
caller's local child remains unset. Models verify termination/drain/handle release
through that path; they are not real kernel SIGKILL or writer-retirement proofs.

The original expired-deadline branch still returns without resume. A complete
recorder therefore explicitly does not prove activation. Operational and telemetry
refusals stay separate, partial exports have no invented endpoints, and an invalid
frame owner refuses before Job creation. Native last-error/clock compatibility
and recorder overhead remain unqualified.

## Evidence and checks

The first 11-case suite, extended 14-case suite, final 15-case suite and combined
130-case regression suite all pass; every first receipt remains. Early receipts
identify their in-progress source inventories. Exact retained implementation
bytes bind to the final model/regression/syntax/check/Clippy inventories; no
earlier source state is represented as the final candidate.

Syntax, xtask all-target check and strict all-target/all-feature Clippy pass.
The preceding whole-workspace formatting command-line refusal remains in the
[offline leaf packet](leaf-attribution-result-20261026.md); it is neither retried
nor converted to a pass here. No Rust product source changes in this stage.

The [42-member packet](local-runs/spawn-frames-20261026/README.md) has index SHA256
`a518934e04231820823c0535c5865c4fe5e19794694fc2b787ea6dc102a7de88`.
Five independent integrity controls pass, including changed predicates/deadlines,
rewritten classified errors and probes moved into the unowned successful-create
gap. The verifier parses source AST and reads hashes/receipts only; it does not
import or execute the retained prototype or tests. A final ledger separately
retains the strengthened integrity, documentation and local contract checks.

The [four-receipt final ledger](local-runs/spawn-frames-20261026/final-checks/README.md)
passes first/strengthened five-case integrity, doc-check and local non-promotable
performance-contract checks. Its index SHA256 is
`f422d27cc85068fe9e7556d24f06100c9e448bb25228b23020722edc5e7c7d28`.

## Next integration gate

Connect the prototype only behind a new explicit gate with its own schema and
exclusive intent/result root. Preserve the old default, outer-only and leaf
modes, common 120-second/five-second origins, terminal checks and original
operational versus telemetry verdicts. Integration needs its own differential
and fault controls before a separately admitted native control. This report
does not authorize that control or an overhead acceptance claim.

No measurement, V6, product workload, dependency change, service/clock change or
server action runs here. The [six release gates](release-closure-status-20261025.md)
remain open. Qualification manifest SHA256 stays
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
