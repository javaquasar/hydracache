# Local creator authenticated root descriptor transfer

Source `65b02f3bac81ef0a29046584d44fc25db95d91b7` adds an ordinary-process
reader and one-shot sender for the existing 104-byte root enrollment request.
Baseline is `f183134cb1e37b7a94c8f60a22f6f868bb98945a`; the previous
[root mechanics evidence](../cgroup-root-enrollment-20261019/README.md) remains
closed and unchanged. This packet concerns descriptor transport, not product
throughput or an original Armed capability.

## Scope and refusal behavior

The actual pair creator supplies the expected PID/TID and real/effective IDs
through local kernel calls. The connected seqpacket pair checks its initial
SO_PEERCRED and enables per-message SO_PASSCRED. The receiver requires exactly
one matching SCM_CREDENTIALS and three rights in directory, kill, events order.
It adopts all installed rights before refusing shape, credential, packet or
role errors. Pipe EOF controls cover rejected extra/truncated rights, rather
than relying on an unstable process-wide FD count. A separate exec process
demonstrates that sharing an endpoint does not make its actual PID the creator.

Root roles require atomic CLOEXEC, exact access modes, cgroup2, root ownership,
the bound directory device/inode, correspondence to named kill/events objects,
and repeated empty unfrozen domain observations. Ordinary files cannot supply
a kernel root. Both operation objects consume before their first syscall;
EAGAIN, peer loss, malformed input or missing confirmation cannot reopen them.

No ACK is emitted. The transferred object exposes no launch, kill, completion
or fence-release method. It is allocator-based ordinary-process code, never a
raw post-clone child entry. Credentials authenticate a local message against
the captured creator, not the original policy/epoch/frame or a live guardian.
Original process-pidfd/deadline/fence custody and authenticated guardian replies
belong to the next integration; these fixtures do not prove owner-death
revocation, a same-attempt reconnect or durable recovery. A privileged host
adversary capable of forging credentials is outside this local primitive.

An empty observation is still not exclusive admission. Future guardians must
remain in revocation mode if writers arrive after owner loss or expiry; a
single kill followed by an empty read cannot close that late-population race.
External writer aliases and sole-collector finalization must also be proved
before durable completion or fence release.

## Implementation files

The new private module is `diagnostic_cgroup_root_transport.rs`, with ordinary
controls in `diagnostic_cgroup_root_transport_tests.rs` and the root/manual
matrix in `diagnostic_cgroup_root_transport_fault_tests.rs`, all under
`tools/long-run-supervisor-074/src`. The old kernel file changes only by its
exact additive module registration. The new xtask source guard is
`performance_contract_074_cgroup_root_transport.rs`; its parent adds only the
forward adapter and module. COMPAT and the gated registry gain exact additive
sections. The separate transport contract and plan record the still-closed
runtime gates. Evidence, collector design, root plan and article are separate
documentation changes.

## Reproduction and receipts

Run `run.ps1 -Compiler 1.94.0 -Label <fresh-label>` and then the 1.88.0 cohort
with another fresh label. Sources must be committed and stay at the same HEAD.
Each cohort records image hashes before and after, source and recipe digests,
nonroot build UID, explicit root fixture UID, empty fixture contexts and the
unchanged active local timesyncd state. Compiler cohorts run sequentially so
their global fixture-quiescence checks are not waived.

Each selected cohort includes check, strict Clippy, thirteen ordinary root
controls (six new transport controls), broad two-phase/exchange regression,
the unchanged seventeen-case redundant guardian matrix, three twelve-case
transport matrices and three eleven-case prior root SIGKILL matrices. The old
kill-caller cuts are regression evidence, not authenticated guardian ACK cuts.
Manual matrices are registered separately; registry discovery does not execute
them. No fixture installs a unit, contacts a server, changes a clock, stops a
supervisor or runs a product candidate.

`inventory.json` hashes exact bytes, including BOM, CRLF and final blank lines.
`verify.ps1 -SelfTest` verifies the closed file set, recipe/source/image scope,
selected matrix summaries, source gate and frozen qualification hash, then
rejects eight mutated inventories. `seal.ps1` refuses an existing inventory;
use a new directory for any later evidence, never reseal this one. This local
inventory is not a signed production receipt or runtime authority.

## Preliminary observations

`red-missing-module.log` is the expected one-test failure before implementation.
An earlier overly exact filter executed zero tests and supplies no red proof.
The first additive source guard failed because its COMPAT removal included one
old final newline; narrowing the exact new addition fixed the adapter without
changing the old body. `source-screen.log` retains the corrected three-control
pass. The uncommitted-source GNU/root screens are not selected receipts.

`registry-check.log` records a misspelled unsupported xtask command; it ran no
registry checks. Only `registry-selected.log`, using `gated-test-check`, is the
accepted registry receipt. Failed commands are not converted to green by
removing their output or relaxing any runtime predicate.

## Selected results

GNU 1.94 and MSRV 1.88 both pass thirteen ordinary root controls (six new),
35 broad two-phase controls, thirty exchange controls and seventeen unchanged
redundant guardian scenarios. Both pass all-target check and strict all-feature
Clippy. Each runs three twelve-case transport matrices and three eleven-case
prior root matrices: 72 selected transport scenario executions and 66 prior
root scenario executions across the two compilers. The latter include 24 actual
SIGKILL cuts of the private kill caller; these are not new guardian ACK cuts.
No fixture roots remain in the recorded closing snapshots and local timesyncd
stays active without any service or clock operation.

Windows passes 54 ordinary controls, eighteen compile-fail controls and strict
affected Clippy. Formatting, registry discovery (184 gates, all registered),
doc-check and the local non-promotable 0.74 performance contract pass. The
previous raw evidence inventory and its eight negative controls pass unchanged.
The qualification manifest raw SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

The complete source suite passes 280 controls in 732.20 seconds. The new exact
forward adapter preserves all 386 prior supervisor Rust sources, both old
guardian drivers and old protocol/registry bodies. No old runtime predicate,
effect bracket, original deadline or qualification byte changes. This closes
the scoped ordinary-process request package, not step four as a whole.
