# Original collector pipe message preparation

This slice prepares an unsent systemd message with the two original output
pipes. It does not connect to systemd or authorize execution. The consuming
entry preserves material, signed Reserved/Starting bindings, installed bundle
descriptors, empty sinks and the original fence in one opaque owner. Its message
owns exactly two known duplicate writers and drops before the original pipes
and fence. Evidence sink descriptors are never message properties.

## Signed settings and lifetime

All 35 fixed worker settings are checked against the exact retained canonical
collector requirements. The separate output requirements remain part of the
same complete projection check. No legacy file-output signature, supplied FD,
unit name, executable path, policy, clock or outcome can be promoted into this
entry. The old UnitProperty, unit builder and transport remain byte-exact.

The two additional properties use actual D-Bus Unix FD values; tests decode the
encoded body and owned FD table, then compare both objects and flags to the
original pipes. Swapped streams, aliased stdout/stderr, a read end and a regular
file refuse. Binary stream bytes remain separate. Closing the creator writer
alone does not produce EOF while the message retains its duplicate; closing
the final duplicate does. The [protocol reference](protocol-reference.md)
distinguishes this representation proof from delivery to an installed manager.

Fresh original opening and closing rounds bracket material/message inspection:
two complete rounds, 34 authentications, in both construction and revalidation.
No ordinary validator or leaf predicate changes. An error latches the entire
chain, and a construction failure retains the owner until drop. There is no
public message/FD extraction, Clone, Deserialize, send, start, retry or completion.

The signed RuntimeMaxUSec is only an immutable requirement ceiling. A future
execution producer must calculate residual absolute deadlines and arm its
independent watchdog before any send; it cannot treat this retained message as
a renewed execution budget. Manager invocation, executable/effective environment,
original process/cgroup, all manager/creator copies and actual worker EOF remain
unproved by this preparation. No new durable record, wire schema or installed
consumer is introduced; the message stays inside one process.

## Verification scope

The scripts retain commands and full generated output for both Linux toolchains,
plus the Windows package checks. Five Linux unit tests cover the representation,
256 seeded projection mutations, every signed property type/name, order and
missing fields, message headers/body, binary streams and last-writer EOF.
The six integration targets cover the new API and the unchanged material,
output join, original pipes, intent and consent contracts. Four compile-fail
examples prohibit sending, extraction, cloning and deserialization.

`contracts.log` retains all 190 static contract passes; `registry.log` accounts
for all 183 discovered gates. `prior-proof.log` runs the unchanged previous
packet verifier at exact source fa680f3d in an isolated disposable source view.
The active branch, user index and old evidence are not rewritten. No new durable
publication boundary is added; this is not another SIGKILL matrix or a substitute
for the previous completed 156-death output proof.

## Retained negative results

The initial API check fails E0599 before implementation. Two unit screens expose
test-expectation mistakes: the signed worker contains 35 properties, not 36,
and zvariant displays the body tuple signature with enclosing parentheses.
Only those test expectations change. `prerequisite-reds.json` explicitly labels
these as excerpts rather than full stdout.

The first local scripts name a nonexistent integration target after their five
passing units. Their full outputs remain `local-first-194.log` and
`local-first-188.log`; the corrected target is diagnostic_original_output_join.
The first static run refuses 17 exact-source guards because the additive registry
block leaves an extra blank line. The complete 9 MB failure output remains in
`contracts-first.log`. Removing that extra line restores all 190 guards; no
frozen input or expected predicate changes.

The first server launch returns 126 because SCP did not set executable mode.
`launch-permission-194.log` retains the actual timeout refusal. The test binary
did not enter libtest and its private attempt directory remains empty. The
corrected runner validates the owned binary path/hash, sets mode 0500,
and uses a distinct final attempt directory. No interrupted protocol attempt is
adopted, and no observed runtime rejection is retried into a success.

## Completed isolated fixture gate

Implementation is `e2e92fd98abbac78ac58a0df149ab665cf3a28a7`, following preregistration
`5a3863148e57b7632121bf5dd84924b0a6f123c8` and baseline
`fa680f3d4220bb0e49bd730c990ff07b9edfe8fa`. Both normal GNU release compilers
pass three complete original chains and all nine independent refusals: six
healthy chains and 18 refusal cases total. The cases are entry-fixture,
entry-build, entry-refused, missing message, later bundle, output, trust,
fixture validation and actual original progress-age expiry. Every refusal
requires its intended error, sticky original refusal, held fence and unchanged
three intent documents. Trust/output failures cannot be accidental time errors.

The healthy families take 40.96 seconds on Rust 1.94 and 40.95 on Rust 1.88;
the nine-case refusal families take 129.87 and 129.84 seconds respectively.
Each includes fresh original-chain preparation, not just message marshalling.
Observed message construction/revalidation phase times, three samples each:

| Compiler | Construction milliseconds | Revalidation milliseconds | Authentications per phase |
| --- | --- | --- | --- |
| Rust 1.94 | 321.565–325.224 | 321.476–325.678 | 34 |
| Rust 1.88 | 321.435–322.989 | 321.182–322.637 | 34 |

These are descriptive correctness-fixture spans including fresh original live
walks and bundle revalidation, not isolated serialization timings, a paired
effect estimate, product throughput or an admitted host-overhead claim.

Both compilers pass five debug and five release units, 41 integration cases,
four compile-fail examples, all-target check and strict all-feature Clippy.
Final checks repeat compilation/lint after adding the pre-refused-owner case;
only the final release binaries are dispatched. Windows current/MSRV package
checks and strict Clippy pass for supervisor and xtask. Workspace formatting
passes in WSL, followed by a final scoped check. No full release verify is run.

The server root is `/tmp/hc074-pipe-transport-20261017.b9l0y9`, owned 1000:1000
mode 0700. Independent postflight verifies no remaining test process, unchanged
boot, supervisor PID 8839/NRestarts 0/start time, installed binary and provisioning
receipt hashes, absent active contexts and absent diagnostic install root.
The original nonexecuted signed bundle bytes are copied into this private root;
no release is installed and no service or clock is changed. The initial mode
refusal directory is still empty. Qualification manifest SHA-256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
The local performance contract passes; require-ship deliberately refuses the
unresolved candidate/qualification. `verify.ps1` checks inventory, hashes,
committed/current sources, frozen leaves, exact family membership and expiry,
with a missing-case canary. Run it from PowerShell after checkout.

## Remaining execution gates

The message prerequisite does not admit production. Independent watchdog arming,
authenticated original send/start, pinned executable and effective streams,
original manager invocation/process/cgroup, collector pumping and all-writer
revocation/EOF remain next. Authenticated completion, cells 2–4 and their real
SIGKILL/lost-ack boundaries follow those producers. Production trust enrollment,
signed installation and integrated nonproduct rehearsal remain separate gates.
The broader historical audit failures, unresolved C74 and release qualification
are unchanged. No supervisor stop, release installation, clock change, product
workload or expensive Actions run is part of this packet.
