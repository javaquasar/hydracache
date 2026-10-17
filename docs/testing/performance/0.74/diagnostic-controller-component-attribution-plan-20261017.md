# Original controller component cost controls

Baseline is `bb2c843f63fb941ea1011bf99221772cfbfc3052`. The unchanged-frame
diagnostic identifies ReservedLive at 131–134 milliseconds with sixteen IPC
authentications. Nonroot sampling is unavailable on this host with
perf_event_paranoid=4; do not change kernel settings or acquire root privileges.

Use existing validators and data-verification functions on fresh original
OIDC/controller/build/worker objects, without changing any production function,
adding instrumentation inside a frozen guard or reproducing its implementation
as an alternative validator. Add only a test-only component-control test and
registration. No controller attempt, Reserved, Starting, output or bundle is
created. These controls run before reservation and are not a direct decomposition
of a ReservedLive call. The private controller capture itself is not timed.

Preregister ten component names: worker-live, controller-source,
signed-controller, builder-body-data-only, build-validator, oidc-source,
oidc-token-data-only, oidc-validator, authority-binding and worker-time.
Signed-controller/build/OIDC validators must make 1/2/4 IPC authentications and
signatures respectively; all other component counters are zero. Builder-body
loads the reviewed policy and verifies both pinned and retained receipts, comparing
their full identities and statements to the original build. OIDC-token uses a
fresh original worker time sampled immediately before the isolated data call,
outside its measurement span, and compares the digest to the original token.
Neither data-only verification creates authority or substitutes for a live validator.

Take three fresh samples on normal GNU release Rust 1.94 and MSRV 1.88.
Use deterministic component order rotations 0, 3 and 6; this is not a fully
counterbalanced effect estimate. Capture each call with the existing IPC numeric
trace, retaining inclusive wall/authentication/signature spans, exact component
membership and successful return. Require original full OIDC validation before
and after the controls, no refusal, unchanged original request and unchanged
worker/OIDC fixture bytes. Preserve any failed diagnostic; do not retry protocol
attempts into success, refresh timestamps or enlarge a window.

Use authorized nonroot isolated temporary server fixtures only, with exact binary
hashes, supervisor/boot/installed-artifact guards and independent executable-object
postflight. No install, service/clock change, product workload, observer or expensive
Actions. Preserve prior evidence at its exact source. Run current/MSRV focused
tests, all-target checks and strict Clippy, static contracts, gate registration,
formatting, documentation and non-promotable local performance checks.

Report all sixty rows separately by compiler, sample, order and component.
Do not add or subtract nested validator and data-only control times, label their
remainder filesystem/CPU cost, or claim an optimization benefit. Their purpose is
to select the next instrumented owner or isolated proposal. Original live checks,
observation frequency, origins, deadlines and execution admission remain unchanged;
grouped material/message/alarm preparation still requires separate agreement.
