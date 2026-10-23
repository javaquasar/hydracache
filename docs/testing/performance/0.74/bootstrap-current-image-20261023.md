# Current executable lookup remains separate from bootstrap authority

The new opt-in --systemd-bootstrap-image-v1 mode checks that procfs executable
lookup and inherited bootstrap FD13 name the same sealed regular inode. A forged
context running from a matching sealed copy still exits 2 with original
authentication closed. Entry-only and complete-census modes remain unchanged.

## Boundary and implementation

Baseline c3dd8469 can compare every inherited role with a sealed entry while
running from another executable. The [plan](bootstrap-current-image-plan-20261023.md)
adds only the current-image leaf and a separate mode, without changing the
original owner, manager, launcher, guardian, parent frame or absolute windows.
The new entry validates the same activation assertions; it never retries an
execution through an older mode.

The order is entry read, complete role census, current-image observation,
unchanged complete census revalidation, then a second fresh current-image
observation. Each image leaf brackets its work with original entry revalidation.
It snapshots readonly FD13 with inherited flags zero and requires its
device/inode/size to match the entry. A fixed procfs thread-self directory must
be a directory on PROC_SUPER_MAGIC. Deliberately following its exe magic link
must yield a readonly regular image with exact seals and CLOEXEC inspection
flags. The complete metadata stamp must equal FD13 before and after inspection;
both offsets must remain zero. The existing census owns content hashing and ELF
validation; the image leaf does not add another content read.

Only successful new inspection opens become owned descriptors. They drop within
the leaf, before the next closed-set census. There is no inherited-role close,
repair, send, runtime exec, timer creation or deadline refresh. The runtime
retains only metadata between leaves, not an extractable execution capability.

These are sequential observations within the process's procfs view. They do
not attest an initial or independently enrolled mount/user namespace, prevent
arbitrary concurrent exec or establish privileged provenance. A substituted
namespace needs its own protected-policy join. Matching entry bytes and image
metadata do not authenticate credentials, signed policy/build, original
manager/root/guardians or every writer.

## Local controls

Unit tests distinguish two sealed readonly files containing identical bytes
but different inodes, require stable repeat observations and reject invalid
FDs, wrong descriptor flags and missing seals without changing flags/offsets.
The integration fixture uses existing private pipes, seqpacket, timerfd,
memfds and owned sleeper pidfds. Only its new sealed diagnostic bootstrap copy
is executable; it is not the original launcher or a product process.

One fresh child executes the same sealed inode supplied as FD13 and reaches
only authentication-closed. Another executes the on-disk binary while FD13
contains its identical sealed bytes and refuses at image consistency; the disk
image also lacks the memfd seals, so this process control is not an isolated
inode-only experiment. The two-sealed-file unit supplies that distinction.
Extra FD and wrong bootstrap digest controls refuse through the preceding
census. The existing entry-only and complete-census tests remain exact.

The [captured first results](bootstrap-current-image-first-result-20261023.json)
retain the initial missing-component source failure and the development E0603
visibility failure. The fix limits the new leaf's visibility to its existing
bootstrap binary root, with no public library API or owner/descriptor export.
These captured summaries are separate from the raw fixed-runner receipts.

## Fixed gate results and evidence

All four fixed lanes completed without failure on the same source snapshot:

| Scope | Passed controls |
| --- | --- |
| Windows | 46 source contracts and 15 bootstrap/unit/process controls, all-target check, strict all-feature Clippy and affected-package formatting. |
| GNU 1.94 | 64 selected unit/process/regression controls, all-target check, strict all-feature Clippy and the exact selected unsigned bootstrap build. |
| GNU MSRV 1.88 | The same 64 selected controls, all-target check and strict all-feature Clippy. |
| Metadata | Three runner controls, doc-check, 188 discovered/registered gates, local nonpromotable performance contract and diff checks. |

Source commit is 15dbb665a1cac461b01e8a1058ad839ea23a1f58. Evidence commit
430446d2 contains the exclusive
[inventory](local-runs/bootstrap-current-image-20261023/inventory.json), whose
SHA-256 is 7bb909a4289c40d9d3bc699fc3289309852d4a37c0ebcd389b7f1015b3b62fdd.
It covers 523 members plus the inventory itself; all 524 staged Git blobs were
checked against their exact working bytes and inventory hashes before commit.
Each lane retains its own exact input source snapshot, command receipts and
raw stdout/stderr. Retention never reruns a test or overwrites a packet.

The selected unsigned GNU build was 1603976 bytes with SHA-256
9b628258e8378a688857442000e943991ec3a6235267d5f8ed36b1bfb0a19607.
This is an observation of the checked dirty-worktree build, not a signed
clean-source receipt or enrollment. Trusted deployment requires a new clean
selected build and its independently approved receipt/policy. The qualification
manifest remains unchanged at SHA-256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

## Admission still required

Image consistency cannot select its own trust. The parent already retains
independently protected bootstrap policy/build selection under the original
owner, but the new child has not authenticated that provenance or enrolled its
procfs/credential namespace. Those joins precede any launcher handoff. Then
all-writer cessation, durable completion, later cells, full SIGKILL/lost-ACK
coverage and separately admitted server rehearsal remain necessary.

No performance measurement, original live retry, clock/service change,
installation, privileged action, server run or product workload is part of
this component. Test elapsed times cannot establish original deadline fit.
