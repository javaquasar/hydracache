# Grouped preparation process loss and return acknowledgement

The grouped nonexecuting candidate is tested at real process-loss boundaries,
not only logical errors. Source is a995170b, baseline is 674ea586 and the
[protocol](../../diagnostic-grouped-process-loss-plan-20261017.md) is
preregistered in 29c7c6fc. Neither the candidate nor production code changes.
The five-file source capsule adds only the private harness, exact registration
adapters, explicit manual gates and a static preservation contract.

All declared cohorts pass on the dedicated host, without installation or service
changes. The matrices are complete for these six selected preparation cuts:

| Cohort | SIGKILL | Ordinary ACK controls | Lost ACK controls | Fresh inspections |
| --- | --- | --- | --- | --- |
| First Rust 1.94 screen | 1 | 1 | 0 | 4 |
| Rust 1.94 three repeats | 18 | 3 | 3 | 48 |
| Rust 1.88 three repeats | 18 | 3 | 3 | 48 |

The screen takes 26.41 seconds and repeated families 310.46/311.13 seconds.
These are harness completion times, not product latency or performance results.
`cases.json` retains fifty exact case rows; raw logs also retain all one hundred
fresh inspections and six repeat summaries. The offline verifier checks exact
membership and ordering instead of trusting only the final summary line.

## Interruption and acknowledgement model

The child completes the original authenticated preparation through sealed
Starting and empty sinks, then uses the unchanged grouped candidate. Six cuts
cover opened, loaded, bound, message, armed and returned-unacknowledged. At each
cut the child publishes bounded private readiness and blocks on its owned
stdin. The parent compares the cut and PID to its exact Child, with a pidfd
opened before waiting, then sends SIGKILL only through that descriptor. Kernel
termination readiness and wait status SIGKILL are required. A readiness record
is a test checkpoint, not a preparation acknowledgement or start capability.

The acknowledged control releases the returned owner, completes unchanged
reference revalidation, drops the owner and emits exactly one private return
ACK before zero exit. The lost-ACK control follows the same healthy path but
explicitly suppresses that ACK, also exiting zero. Interrupted children must
emit none. ACK grammar rejects foreign PIDs, duplicates, malformed termination
and invalid text. This models return-acknowledgement loss only; no systemd
StartTransientUnit request, SCM_RIGHTS delivery or manager acknowledgement occurs.

Seed 0x07406c17 orders the six SIGKILL cuts as returned-unacknowledged, bound,
message, opened, armed, loaded. Each of three repeats then runs one ordinary
and one lost-ACK control. The original output-join harness remains byte-exact
after removing only its two-line child registration. Its scratch ownership,
snapshot, pidfd cleanup, retained-history and full output-inventory routines
are reused, not replaced by weaker validators. The new constructor/finish
wrappers select the grouped child and validate the distinct ACK result.

## What recovery must refuse

While the child holds the owner, the parent requires the original host flock
to be busy and independently validates complete sealed intent/Reserved/Starting
records and empty output inventory. Original ancestry, context and parent-owned
authority snapshots include metadata and bytes. After confirmed child exit,
the lock must be acquirable and original/authority snapshots unchanged. SIGKILL
retains the interrupted context exactly; normal exits clean temporary context.

Two fresh selected processes inspect every outcome. Retained history must refuse
a new empty-fence context without changing any snapshot. Missing ACK, successful
return or original process death cannot authorize adoption, replay, repair,
completion or later-cell execution. Test-harness waits are fail-safes; no original
admission time origin or deadline is renewed. The process contains only
nonexecuting preparation: this is not P2/P3 external-writer revocation proof.

## Checks and historical evidence

Both normal GNU release compilers, Rust 1.94 and MSRV 1.88, pass four release and
four debug alarm/kernel/reference units, the new two codec units and inactive
child entry, the unchanged output-join protocol unit, six trace units, 41 targeted
integration cases, five compile-fail boundaries, all-target check and strict
all-feature Clippy. Windows supervisor/xtask check and strict Clippy pass. All
195 static contracts pass. The registry validates 183 discovered gates; the two
new nested manual libtests are explicitly registered and independently listed
by both release binaries rather than counted as extra integration targets.

The prior grouped packet passes its unchanged verifier in an archive of exact
source 674ea586. That helper uses an owned index copy, disables optional refresh,
proves the unchanged qualification Git blob before materializing pinned checkout
bytes and verifies the active index hash stays unchanged. No historical verifier,
expected result or qualification input is rewritten.

The local verifier checks exact case/inspection membership and seeded order,
signal/ACK classifications, derived rows, source capsule/blobs, file inventory,
binary/script digests and host guards. Its in-memory control accepts one valid
screen and refuses eight mutated records. The first self-test fixture mistakenly
included an unmodified input because of PowerShell concatenation precedence;
that harness-only failure is retained, and the corrected fixture passes without
changing any process result or candidate code.

The first final verification also refuses differing shell output order for the
same nine fixture hashes. The retained log shows no digest/name drift; comparison
now canonicalizes the digest/name tuples instead of depending on shell locale.
Neither correction reruns a process case or changes a frozen expected outcome.

## Host isolation and remaining execution obligations

The nonroot driver root is /tmp/hc074-grouped-process-loss-20261017.LNpZoJ,
owner 1000:1000 mode 0700. The unchanged scratch harness also creates owned
0700 direct /tmp leaves for its origin/context and authority, as required by
the fixed-ancestry fixture. These are isolated synthetic roots, never actual
/etc or /var/lib configuration or a global deletion target. The signed bundle
is read as fixture data and never executed.

Exact before/after guards preserve boot, supervisor PID 8839/start/NRestarts 0,
installed binary/receipt hashes, absent campaign/context markers and absent
diagnostic installation. No release installation, supervisor stop, root/clock
mutation, observer, product workload or expensive Actions occurs. Qualification
manifest raw SHA-256 stays
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

Independent executable-object postflight finds no remaining test binary,
including deleted executable names, and all nine signed fixture-data hashes
match setup. Workspace formatting, documentation and the local non-promotable
performance contract pass. No full milestone or require-ship gate is claimed.

The remaining production obligations are a reviewed fixed-origin composition,
autonomous guardian lifetime, authenticated original send/start and all-writer
cessation before completion/later cells. This packet cannot establish arbitrary
in-syscall crash or power-loss behavior, production delivery/ACK recovery,
installation, pilot/qualification admission or a product performance claim.
