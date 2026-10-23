# Installed entry stays with the consumed original owner

A private consuming factory now materializes HCBI0741 under the same retained
InstalledBootstrapUnit. It reads the original sealed entry, derives the installed
origin assertion and retains one new sealed record. No caller record, alternate
owner, descriptor getter, child mode or transport is accepted. The production
ownership path compiles; a positive live original-owner join remains unproved.

The new Rust modules are diagnostic_bootstrap_installed_entry_owner.rs,
diagnostic_bootstrap_installed_entry_resource.rs and
diagnostic_bootstrap_installed_entry_census.rs, each with a paired test module.
The change also adds an exact source/COMPAT guard, extends the existing xtask
source-contract target and adds the fixed bootstrap-installed-material-074
check/retention tool. No dependencies or gated-test registry entries change.

## Derivation from retained resources

The [local plan](bootstrap-installed-material-plan-20261023.md) extends the
[assertion codec](bootstrap-installed-entry-20261023.md) without changing its
bytes. The resource leaf checks the original EntryResource stamp, readonly
access, CLOEXEC, all four seals and zero offset. Exact prior census comparisons
bracket reading its 1024 bytes. The installed census must preserve all old
27 FD values and fifteen roles before deriving the 1280-byte envelope.

The parent fingerprint uses a separate domain and fixed-order roles/FD values,
original-entry stamp, installed-image stamp and four ancestry stamps. It is
an assertion checksum, not a signature, transferable capability or child FD map.
The same verified build and retained source remain checked by the old complete
owner frames; the new leaf does not independently enroll trust or file ancestry.

The existing sealed constructor creates the record. Its writable construction
alias closes before publication, leaving a readonly/CLOEXEC/four-sealed memfd
with zero offset. New inspection checks exact content and retained metadata,
flags and offset, derives expectations again and compares them before returning.
Parent inventory preserves its 32/16 prefix and appends one record, reaching
33 physical FDs and seventeen semantic roles. Every old FD/inode alias refuses.
The embedded counts remain 32/16: including the new record in its own fingerprint
would introduce a circular definition.

## Refusal and lifetime

The wrapper reuses the unchanged validator: an old installed complete frame,
new inspection, old complete frame, new inspection and old closing frame. All
old checks and 300/60/10 origins/windows remain; additional work is not a fresh
allowance. Missing resource after retention refuses instead of recreating it.
Resource mismatch is sticky even after restoration. Wrapper failures latch the
same owner, and the error retains that refused owner and any published record.

Drop explicitly refuses the original before retained descriptor closure; field
order also keeps the owner before the new resource. Existing internal sealed
constructor temporaries keep their old error cleanup behavior. There is no
claim that transient construction handles remain open after every kernel error.
The memfd is not durable completion or an on-disk lifecycle transition.

## Local coverage

Five private kernel-resource controls exercise exact bytes and offsets,
write/truncate refusal, repeated inspection, content fault injection,
same-byte replacement, flags, metadata, original substitution, wrong prefix and
refusal after restoration. Two census controls check exact extension, aliases,
size/seal/offset refusal, prefix mutations and fingerprint sensitivity. Three
owner controls model full-frame order, every cut, drift, missing-resource
refusal and the compile-only consuming production signature.

These use synthetic images/claims and private temporary resources, not real
root-host enrollment or original manager/guardian execution. Models do not
prove all-writer cessation or the positive live owner path. Exact source views
remove only three module hooks; old frames/kernel bodies, record formats,
child modes and qualification inputs remain intact.

The [first outcomes](bootstrap-installed-material-first-result-20261023.json)
retain the absent-owner baseline and the initial GNU/MSRV strict lint refusals.
All ninety selected controls and compilation passed in those first Linux lanes;
Clippy then rejected a redundant test-only re-read immediately after restoring
an FD slot. Reusing the first saved value removes that almost-swapped pattern
without suppression or runtime changes. Final tests additionally reject corrupt
old padding/magic and zero/mismatched installed size in the value factory.

All four final lanes passed on one stable 128-file source snapshot:

| Scope | Passed checks |
| --- | --- |
| Windows | 60 source contracts and 22 preceding codec/bootstrap controls, all-target check, strict all-feature Clippy and affected-package formatting. |
| GNU 1.94 | Ten new resource/census/owner controls plus eighty preceding controls; all-target check, strict all-feature Clippy and exact selected unsigned build. |
| GNU MSRV 1.88 | The same ninety selected controls, all-target check and strict all-feature Clippy. |
| Metadata | Four Python controls, doc-check, 188 discovered/registered gates, local nonpromotable performance contract and diff checks. |

Core source commit is 46426601; checked source/tooling commit is
9bdaeb6f747d596ba38289f6487d8a5a016bd3dc. Evidence commit 19874664 retains the
exclusive [inventory](local-runs/bootstrap-installed-material-20261023/inventory.json),
SHA-256 fa919e682a77aa0eeed7c6cea92f38848df8f224e72917e9e11496697a39c60a.
All 1337 members plus inventory were verified against 1338 exact staged Git
blobs before commit. Initial and final snapshots/logs remain separate.

The selected unsigned GNU check image is 1607368 bytes, SHA-256
00f5f0ff4f27cbd11c070e0174f76e124971b4622cc6bae88d6b1ad86b5e098f.
It is a dirty-worktree check build, not a trusted clean-source signed receipt,
installed origin enrollment or numerical performance result. The qualification
manifest remains unchanged at SHA-256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

## Remaining child boundary

The next component needs a separately versioned child inventory/current-image
reader distinguishing the installed executable from the sealed reference. Actual
sandbox credentials, namespaces, process generation, streams and cgroup must
then be admitted through the same original manager before release. Authenticated
delivery, writer cessation, durable completion, later cells and full real
SIGKILL/lost-ACK coverage remain integration gates. Added preparation cost has
not been measured against the original live windows.

No clocks, services, installation, server actions, original live attempt,
product workload or performance cohort are part of this component.
