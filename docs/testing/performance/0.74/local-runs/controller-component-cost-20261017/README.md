# Original controller component cost controls

All sixty separate component controls pass on fresh original objects: three
samples each on normal GNU release Rust 1.94 and MSRV 1.88. Data-only builder
verification takes about 0.10 milliseconds and OIDC token verification about
0.03; full build and OIDC live validators take about 13 and 30 milliseconds.
The next profiling target is repeated worker/kernel/authority observation, not
an assumed expensive immutable signature. No optimization is accepted here.

Baseline is `bb2c843f63fb941ea1011bf99221772cfbfc3052`. The
[protocol](../../diagnostic-controller-component-attribution-plan-20261017.md)
is preregistered in `7fe4d8c9`. The implementation source and four-file source
capsule are pinned in `manifest.json`; every production validator body remains
byte-exact. Only a test module, registry and static contracts change.

## Scope of the measurements

These are separate calls before reservation, not an instrumented decomposition
of one ReservedLive operation. The private controller-capture function is not
called or timed directly. The tests invoke existing live validators and the
same data-verification functions on authenticated original objects, without
copying their implementations, widening visibility or replacing a validator.

Each sample uses deterministic order rotation 0, 3 or 6 across ten components.
This is not a fully counterbalanced comparison or product workload. Full OIDC
validation succeeds before and after the controls. The original request and
worker/OIDC fixture bytes remain unchanged, with no original refusal. The signed
controller, build and OIDC controls require exactly 1, 2 and 4 IPC authentications
and signatures; all other controls require zero. Each numeric row is printed
only after successful return and counter checks.

Builder-body-data-only loads the reviewed policy, verifies both pinned and
retained receipts, and compares complete identities and statements to the
original build. OIDC-token-data-only samples actual original worker time just
before its separate verification span, and requires the original token digest.
That clock sampling is excluded from the token's span. These data-only results
are not authority, successful live admission or a substitute for their full
validators. No request deadline is refreshed.

## Observed inclusive fixture spans

Three samples per compiler, milliseconds rounded to three decimals:

| Separate component | Rust 1.94 | Rust 1.88 | IPC authentications |
| --- | --- | --- | --- |
| worker-live | 1.504–1.507 | 1.496–1.507 | 0 |
| controller-source | 0.019 | 0.019 | 0 |
| signed-controller | 6.503–6.521 | 6.470–6.532 | 1 |
| builder-body-data-only | 0.101–0.103 | 0.102–0.105 | 0 |
| build-validator | 13.101–13.154 | 13.035–13.133 | 2 |
| oidc-source | 0.021 | 0.020–0.021 | 0 |
| oidc-token-data-only | 0.029–0.033 | 0.030–0.031 | 0 |
| oidc-validator | 29.727–29.809 | 29.575–29.835 | 4 |
| authority-binding | 3.052–3.066 | 3.040–3.057 | 0 |
| worker-time | 3.418–3.426 | 3.397–3.416 | 0 |

Both three-sample test families complete in 0.70 seconds, including setup.
`phases.json` retains all sixty exact rows separately by compiler, sample and
order. `verify.ps1` checks raw membership, rotation, exact row values, counters,
six completed controls, source capsule/committed blobs, inventory hashes and
unchanged qualification identity. It does not execute a server workload.

The controls narrow the source hypothesis. The full validators nest other
validators and fresh worker/kernel checks. Their times must not be added to
or subtracted from the data-only controls to manufacture an exclusive call-tree
profile. IPC counters do not cover every cryptographic operation. The measured
contrast does not prove a filesystem/syscall/CPU owner, estimate a speedup or
justify caching mutable trust, time, authority or peer state. Further attribution
should examine original worker/kernel/authority walks with all current observations
and deadlines retained. Grouped material/message/alarm preparation is still a
separate agreement and healthy/fault gate.

## Checks and unchanged historical proof

Rust 1.94 and 1.88 pass four release and four debug kernel/reference tests, six
trace units, 41 integration cases, five compile-fail boundary examples, all-target
check and strict all-feature Clippy. Windows supervisor/xtask checks and strict
Clippy pass. All 193 static contracts pass, including exact original validator
bodies; all 183 discovered gates are registered. Formatting, documentation and
the non-promotable local performance contract pass. Require-ship remains an
expected refusal; no full milestone release verification is claimed.

The prior leaf packet passes its unchanged verifier at exact source bb2c843f.
The helper uses an owned archive view, proves the frozen qualification Git blob
before materializing its pinned Windows checkout bytes, uses read-only Git
commands and verifies the active index hash is unchanged. The prior packet and
its expected values are not rewritten. The perf capability denial is a tooling
observation in `setup.log`, not a failed product candidate or permission to alter
kernel settings.

## Host isolation and remaining execution gates

All server writes are nonroot inside `/tmp/hc074-controller-cost-20261017.uYyrhr`,
owner 1000:1000 mode 0700. `setup.log` records perf_event_paranoid=4 and actual
`perf stat -e task-clock -- true` refusal with exit code 255. No capability,
kernel setting or service confinement is changed. No bundle or observer is
needed. No attempt, Reserved, Starting, output, message or alarm is created.

Independent postflight finds no remaining test executable via `/proc/PID/exe`,
including deleted executable names. Boot, supervisor PID 8839/NRestarts 0/start,
installed binary and receipt hashes remain unchanged. Active campaign/fixture
contexts and the diagnostic install remain absent. No release installation,
supervisor stop, clock change, product workload or expensive Actions run occurs.
Qualification manifest SHA-256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

The alarm candidate remains rejected/test-only. Autonomous guardian lifetime,
authenticated original start, all-writer cessation, completion, later cells and
their new SIGKILL/lost-ack matrix remain open, alongside actual enrollment,
installation, integrated nonproduct rehearsal, historical audit blockers,
C74 identity and release qualification.
