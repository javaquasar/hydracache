# Original owner and manager sandbox observation binding

A private consuming constructor now binds read-only sandbox observation to the
same InstalledEntryUnit and its original FixedManagerChannel. It takes no PID,
manager, unit name, credentials, namespace, image, stream or timestamp arguments.
The backend compiles and its local controls pass; positive original-manager
sandbox integration and authenticated child enrollment remain unproved.

The [local plan](bootstrap-sandbox-owner-plan-20261023.md) adds three modules:
diagnostic_bootstrap_sandbox_owner.rs for ownership and full-frame composition,
diagnostic_manager_sandbox_query.rs for fixed original-channel queries, and
diagnostic_sandbox_process.rs for retained kernel observations. Four exact
additions connect them to existing code and derive installed image identity as
values only. No original descriptor getter, dependency, CLI mode, durable format,
configuration or gated-test registry entry changes.

## Expectations come from the same owner

The constructor derives the unit, attempt root, cgroup path, boot, original
origins/bounds, stream identities and manager/client names from the retained held
projection. Installed image identity comes from the same validated installed
census. Child UID/GID and supplementary groups come from the same authenticated
worker-policy document, not the root creator credentials in the entry assertion.
Parsing this document does not independently select an issuer or renew trust:
unchanged complete original frames authenticate its exact retained bytes and epoch.

The wrapper brackets two manager/kernel observations with three existing complete
InstalledEntryUnit validations. It compares the entire derived source, process
PID, nonzero invocation and unit object path without refreshing a retained pin.
Any failure refuses the same original owner. Constructor errors retain that
owner and any published observation; explicit refusal and field order precede
published observer closure. Internal failed-construction read-only temporaries
can close without granting or releasing execution rights.

## Queries and kernel identity

GetUnit and nine fixed property reads use the retained unique manager on the
same authenticated connection. The query verifies reply sender/destination/type,
signature, bounded bytes and absence of attached rights; zbus correlates reply
serials. Exact Id, nonzero InvocationID, active/running state, Transient, matching
MainPID/ExecMainPID, attempt-scoped ControlGroup and successful Result are required.
The original channel revalidates before and after the bounded operation. Missing
unit, wrong type, timeout, disconnect or drift poisons the channel and owner;
there is no replacement connection or retry.

The manager-selected PID supplies the process generation, never a caller's
start-ticks claim. Retained proc documents and pidfd checks bracket acquisition
and later reads. Boot, process group and exact cgroup path must remain stable.
The reader pins user, PID, time and mount namespace objects in nsfs. User/PID/time
must match the original observer; sandbox mount must differ and remain the same
attempt-scoped object. The old same-host mount observer is unchanged.

Before handoff, the held bootstrap must have exactly its leader task. The strict
existing credential parser checks all UID/GID slots, exact groups, zero active
capability sets and NoNewPrivs. Bounding set is captured and compared, not required
to be zero: the existing unit and task contract do not clear or require an empty
bounding set. Review corrected that overconstraint in the unpublished new reader;
no original credential predicate, unit setting or acceptance threshold changed.

The actual executable must match the retained installed device/inode/size and
stable metadata; original full frames retain ancestry/build/content validation.
Readonly O_PATH stream observations must identify the original FIFO objects,
with unchanged writer flags and matching fdinfo inodes. The attempt root inode
must match the retained root, its direct population must be empty, and its only
child cgroup must be this unit. That node must contain exactly the selected PID,
with no descendant directories; root/node type and population/freeze events
must agree. Bounded enumeration and repeated process observations fail closed.
These are sequential observations, not an atomic or continuous kernel proof.

## Local checks and retained outcomes

The [first-result record](bootstrap-sandbox-owner-first-result-20261023.json)
distinguishes the raw absent-component baseline from development tool summaries.
The baseline exited 101 before implementation. Development compilation rejected
two ambiguous filesystem-magic casts, corrected to libc::c_long. New source
assertions needed rustfmt whitespace/trailing-comma handling; a new accessor
needed multiline formatting. Exact old source views were not weakened.

Four owner models check full-frame order, every error cut, restoration refusal,
source/manager/PID/invocation/object-path drift and the consuming production
signature. Two manager controls check target/property and reply-envelope rules.
Three kernel controls exercise bounded parsing, real owned-helper generation and
pipe-target reads, and foreign-cgroup/same-host-namespace refusal. The helper is
always killed/reaped by its own RAII guard. No fake procfs/cgroup can be promoted
to a positive managed sandbox. Model positives are not production positives.

All four fixed lanes passed on one stable 153-file source snapshot:

| Scope | Passed checks |
| --- | --- |
| Windows | 66 source contracts and 24 preceding codec/bootstrap controls; affected all-target check, strict all-feature Clippy and formatting. |
| GNU 1.94 | Nine new sandbox controls plus 103 preceding controls; all-target check, strict all-feature Clippy and exact selected unsigned build. |
| GNU MSRV 1.88 | The same 112 selected controls, all-target check and strict all-feature Clippy. |
| Metadata | Four Python controls, doc-check, 188 discovered/registered gates, local nonpromotable contract and diff checks. |

Plan commit: cdc75eaf. Core source: 6ab8e415. Checked source/tooling commit:
7958182ce35e557ca1ecd49a9d5f990903784000. Evidence commit f5c8088d retains the
exclusive [inventory](local-runs/bootstrap-sandbox-owner-20261023/inventory.json),
SHA-256 f2c510d011af7dae88acb9fe61632bcc8c66c2951cdf4caf7bdcc2aec70dd26d.
All 786 members plus inventory were verified against 787 exact staged Git blobs.
The [runner](../../../../tools/bootstrap-sandbox-owner-074/README.md) refuses
overwrite, changed sources/commands/logs, incomplete lanes and qualification drift.

The unsigned selected check image is 1639368 bytes, SHA-256
534c253e6bd3c27352376f64856b33d51ad54a1ea938d4ac019fe9dfd7f0b138.
It was built from the checked worktree before commit, not from enrolled clean
signed production provenance. The qualification manifest remains unchanged at
SHA-256 11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

## Admission remains closed

A matching pre-existing manager unit does not prove this owner originally sent
its creation request. This object therefore offers revalidation only, never
start/send/release, writer revocation, completion or fence release. The current
child consistency modes still refuse authority and exit 2. This component reads
identity/status properties, not a complete loaded-unit hardening/ExecStart proof.

The next integration needs an actual held child, complete loaded-unit/FD-role
verification and a bounded authenticated one-shot child enrollment exchange tied
to original delivery, owner/channel, attempt, epoch and original deadline. That
positive path, disconnect/lost-ACK and SIGKILL controls, all-writer cessation and
later lifecycle cells remain separate gates. Under the current no-service scope
there was no positive original-manager sandbox run and no new service/cgroup or
namespace creation. No clocks, existing services, installation, server actions,
product workload or original live/measurement attempt changed.

Additional queries, hashing and full validations have not been measured against
the original windows. The old 300/60/10 origins and bounds remain exact; extra
work is not a fresh allowance and may refuse on expiry. Correctness-test elapsed
time supplies no speed or deadline-fit claim.
