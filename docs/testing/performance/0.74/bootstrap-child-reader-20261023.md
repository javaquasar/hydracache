# Bootstrap executable reads assertions without authenticating original start

The separately named diagnostic_held_bootstrap executable now has a bounded
HCBT0741 reader. It is a refusal-only boundary: even a forged canonical sealed
record exits 2 at original-authentication-closed. Parent ownership/transport,
original role authentication, launcher execveat and writer cessation are not
enabled by this executable.

## Audit boundary and implementation

Baseline c9654c84 materializes the parent's entry record but has no bootstrap
binary. The [component plan](bootstrap-child-reader-plan-20261023.md) adds that
binary without weakening any existing original frame or absolute 300/60/10
window. Source contracts compare complete old owner, material, projection,
launcher, activation, manager, guardian, build and entry-code bodies against that
baseline. Lockfile, package manifest, gate registry and frozen qualification
remain exact. The new binary is discovered by Cargo, not installed or activated.

The explicit argument and three exact activation fields are registered in
[COMPAT](../../../COMPAT.md). PID is canonical decimal; count is twelve; names
preserve the old slots and propose host/launcher/bootstrap-image/entry at
11/12/13/14. Only FD 14 is inspected. Matching names/count does not authenticate
the other inherited descriptors or establish that systemd sent them. Unknown,
duplicate, future, missing or extra input refuses without environment cleanup,
flag repair, fallback or retry.

The reader uses no File ownership conversion, reopen, exported descriptor or
clock source. It requires a regular readonly/non-O_PATH descriptor, exactly four
seals, size 1024 and offset zero. Child descriptor flags must be zero because
parent CLOEXEC would otherwise close it across exec; the reader does not change
those flags. Bounded positional reads preserve shared offset. It rejects EOF,
errors including EINTR, malformed shape and metadata/status/seal/offset drift,
then compares a second fresh complete observation. This is point-in-time input
consistency, not atomic/continuous authority or a wall-time execution guarantee.

The next boundary remains closed unconditionally. No credential, original
manager channel, guardian/root context, image contents, complete inherited census
or signed actual build is authenticated here. The private observation type has
no owner/FD/permit extraction. No start/send/execveat call exists. All old live
checks remain in their original path, not replaced by this isolated reader.

## Test evidence

The [first absent-binary failure](bootstrap-child-reader-first-result-20261023.json)
is a captured-tool summary, not a raw fixed-runner receipt. Reader units use real
private memfds; original context fields are synthetic assertions. Fresh CLI
controls execute only the reader with a test-only inherited descriptor. The
harness duplicates that private descriptor in the child; this is not original
parent-owner transport. Unknown input and absent/malformed entry refuse. A valid
forged record reaches only explicit authentication-closed, never a worker.

All fixed lanes passed on stable checked sources:

- Windows: 38 current-source contracts, three ingress units, one CLI control,
  two entry codec and five signed-assertion units; all-target check, strict
  all-feature Clippy and formatting.
- GNU 1.94 and MSRV 1.88 each: seven new ingress/reader units, two fresh CLI
  controls and 37 preceding entry/census/build/owner/source regressions, 46
  selected tests per compiler; all-target check and strict all-feature Clippy.
  Default-feature check retains the preceding unused Reading::Reference warning.
- Three runner/retention controls, doc-check, 186 registered gates, local
  non-promotable performance-contract-check and diff check.

GNU also passed the selected fixed no-default-feature build command. Its
unsigned output is 1553056 bytes, SHA256
`784be7a51e66f5f3b38bbb9cfdac09c763ceaece6d791582274a2ec8b26a7a47`.
This build uses the checked worktree; it is not a clean-source signed receipt,
production policy selection or bootstrap enrollment. Rebuild/sign through the
actual selected trusted source before enrollment. Unit/build elapsed times are
not performance evidence.

The [exclusive packet](local-runs/bootstrap-child-reader-20261023/audit.json)
checks source commit `860c4edd`; evidence commit is `568f7a4f`. Its
[inventory](local-runs/bootstrap-child-reader-20261023/inventory.json) lists 212
members excluding itself, SHA256
`16369423a683b36dc94fcf119db0a5c91babbcb32171dee1ddab5d6bed9c473d`.
All 213 staged Git blobs including inventory were checked against retained sizes,
hashes and worktree bytes before committing. Exact command/output hashes and
source-before/after receipts remain separate from the captured first-failure
summary. Plan commit is `43f628c0`; scoped check tooling is `ea3e1fdb`.

Qualification SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

No original live retry, measurement, service/clock change, systemd request,
server action, installation or product workload occurs. Previous first failures
remain in their existing packets. Qualification input bytes are unchanged.

## Remaining authenticated execution gate

Validate every inherited role and actual executable/streams/process/cgroup,
credentials and namespace, original manager/root/guardian context and absolute
deadline. Bind these checks to the original consumed owner and independent
protected bootstrap selection. Enroll the exact signed build before allowing
one-shot execveat of the same sealed launcher. Then prove all-writer cessation
and SIGKILL/lost-ACK boundaries before durable completion, later cells,
reconnection or server admission. No reader outcome substitutes for those gates.
