# Redundant watchdog custody and process fault evidence

This local package composes two unchanged private guardians to test custody and
target revocation under one guardian failure. It is not original execution
admission, a production topology decision or whole-cgroup/all-writer proof.
The [plan](../../redundant-watchdog-plan-20261019.md), committed as `b1825bdd`,
starts at `6f1dbfab`. The [contract](../../redundant-watchdog-contract.toml) keeps
production construction, server operation and qualification closed.

## Ownership and uncertainty

The pair owns two guardians, each authenticated to the same creating process.
Both Pending acknowledgements precede target creation. A clone of the original
fence File preserves the same flock OFD; the children isolate all other inherited
descriptors and independently retain their fence aliases. Both original
hc074-prearmed-guardian-v1 instances receive exactly the same selected absolute
BOOTTIME deadline. No relative lease, restart or replacement is introduced.

The pair consumes global uncertainty before the first enrollment send. Each
guardian retains the same target pidfd before its own Armed; both Armed and a
closing live check are required before the fixture sends the unchanged barrier
packet. Failure at either ACK poisons the whole pair. No sender retry, public
constructor, original artifact, evidence sink, completion or unlock is available.
The actual target writes one constant byte to a pipe and parks without further
owner/clock monitoring, so released-target revocation must come from a guardian.

## Seventeen process controls

The external nonroot subreaper uses actual creating-owner credentials, exact
owned pidfds and parent/executable identity. It checks real SIGKILL status, pipe
EOF, retained fence aliases and unchanged fence inode/metadata. Cleanup kills
and reaps only its direct/adopted libtest children. Seed is `0x0740d019`.

The cases cover healthy expiry and owner loss; death or SIGSTOP of either
guardian followed by creator SIGKILL; death or SIGSTOP of either guardian with
the creator live through the original deadline; lost second ACK with the first
guardian dead or stopped; partial enrollment loss and retry refusal; global
duplicate refusal; and a hidden writer with one guardian dead.

The partial-enrollment fixture stops the second guardian after target retention
but before Armed, then the driver kills that guardian. The first still owns the
unreleased target; the fixture asserts sticky global refusal and retry denial,
and the driver verifies no output or duplicate-message revocation before actual
creator death. The two lost-ACK cases instead kill the creator at that same cut
and resume the surviving guardian; no release effect may occur.

Two expected-negative controls are required. Killing both guardians leaves the
released target and writer live with the final flock free. SIGSTOPing both,
then killing the creator, leaves the writer live at the original deadline until
one guardian resumes; the fence stays held. A hidden external FD copy separately
prevents EOF after target SIGKILL until the driver closes that alias. These are
limits of this local fault model, not successful all-writer or completion gates.

## Baseline and preliminary receipts

`baseline-units.log` passes the prior fourteen ordinary two-phase controls;
`baseline-matrix.log` repeats the prior twelve process cases with their original
negative controls. Its image hash matches the previous selected GNU image.
`red-module.log` records the expected missing-module failure before implementation.
`units-first.log` retains a wrong test-module path to the clock function; only
that new fixture path was corrected. `units-corrected.log` passes the four new
ordinary controls. `first-dead-owner-first.log` and `matrix-gnu194-first.log`
pass one initial real fault and the full seventeen cases before source selection.
`source-first.log` passes the three new source guards. No old predicate, timing
bound or protocol was changed to make these checks pass.

## Selected implementation and reproduction

Selected source is `c3539d171118d796812a3d1dbad16cfdb4296f6e`.
The GNU 1.94/MSRV 1.88 cohorts bind that same source, compiler, nonroot UID and
equal before/after binary hashes. Each runs all-target checking and strict
all-feature Clippy, eighteen ordinary two-phase controls, the prior twelve-case
barrier, the unchanged twelve-case single-guardian matrix, the new seventeen-case
pair matrix and thirty old exchange controls. Ignored fixtures are not counted
as executed. These are correctness checks, not throughput/allocation measurements
or a full release milestone/qualification.

From the repository's Windows/nonroot WSL workstation with clean selected source:

```powershell
./run.ps1 -Compiler 1.94.0 -Label fresh-gnu194
./run.ps1 -Compiler 1.88.0 -Label fresh-gnu188
```

Use the script's full path or run it from this evidence folder. It refuses dirty
selected source, existing cohorts, empty/failed gates, source/image drift and
changed qualification bytes. New reproduction directories extend the inventory;
do not overwrite old receipts or present an extended directory as this packet.
`./verify.ps1 -SelfTest` verifies the closed canonical UTF-8/LF/no-BOM inventory,
the committed-source cohort bindings and eight in-memory negative controls.
Actual image SHA-256 values are retained in each receipt and matrix header.
`reproduction-guards.log` checks script syntax and three nonexecuting refusals:
an existing cohort, path-escaping label and unsupported compiler. Separate
inventory receipts verify both preceding packets without changing them.

Separate local receipts cover Windows portable/API controls, strict affected
lint, formatting, full source contracts, registry, doc-check and the local
non-promotable contract. Windows passes 49 portable units and 17 compile-fail
controls; the full source suite passes 268/268. New source guards restore all
369 old supervisor Rust bodies, old compatibility/registry and both imported
Python harnesses exactly; mutation controls reject changed old enrollment
predicates or registrations.
The frozen qualification manifest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

## Changed files and remaining admission gates

Implementation is `diagnostic_redundant_watchdog.rs`; ordinary controls and the
owned subprocess are `diagnostic_redundant_watchdog_tests.rs` and
`diagnostic_redundant_watchdog_fixture.rs` beside it. Prior guardian/fixture
files receive exact additive child-module registrations only. The new external
driver is `tests/redundant_watchdog.py`; old imported drivers remain unchanged.
`performance_contract_074_redundant.rs` is registered through the previous
prearmed source guard. Compatibility, manual registry, local contract, plan,
collector design and optimization article record scope and results.

This pair does not instantly detect a stopped sibling; with a live creator it
may revoke only at the original deadline. It cannot survive both holders dying,
host failure, cgroup escape or unknown external writers. Its local procfs and
direct-child checks cannot authenticate historical original admission or a
systemd-managed launcher. Preserve existing original collector/custodian owners,
join trusted original kernel/manager/artifacts/process/cgroup/streams inside the
full validation frame, and prove all writers ceased before completion or release.
Durable history, completion/later cells, complete new-boundary crash/lost-ACK
coverage and monitoring reconnect remain later gates. Original live checks,
46 effect brackets and 300/60/10-second origins are unchanged. No server,
installation, supervisor operation, original/system-bus send, product workload
or expensive qualification occurred here.
