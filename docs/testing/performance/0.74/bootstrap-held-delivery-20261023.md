# Original held delivery and child readiness components

The private parent delivery candidate is implemented under the consumed original
installed-entry owner. The separate one-shot readiness primitive works with
owned local helper processes. This is not a completed authenticated original
start: the new eighteen-role child entry and exact activation environment remain
unconnected, and no original-manager service delivery was attempted.

## Implementation

`diagnostic_original_held_delivery.rs` retains the same owner, original context,
installed-entry digest, request and socket pair. Preparation, delivery and peer
exchange keep unchanged original full validations. The existing sandbox
observation additionally supplies the selected PID, invocation, worker policy,
installed executable and original streams before the readiness challenge.
Any error refuses the same retained owner and transport state; no replacement
owner, frame or admission window can be supplied by a caller.

`diagnostic_held_delivery_wire.rs` constructs a distinct held request. All
projected properties remain, with only the fixed bootstrap ExecStart and explicit
descriptor properties added. Mode is fail and the auxiliary list is empty.
The original serial and unique manager/client names remain bound. Three stdio
FDs and fifteen named extras produce eighteen distinct retained CLOEXEC aliases;
fresh source/alias metadata, flags, offsets, body and headers must match. The old
parent's 33-descriptor prefix is unchanged. These new aliases and the two socket
endpoints are scoped transport ownership, not an eighteen-slot child census proof.

`diagnostic_manager_held_delivery.rs` registers a bounded reply stream before
sending the exact retained message on the same validated connection. It never
rebuilds a request through a generic call API. A correctly correlated job reply
must name the original manager, client, serial and canonical job path. Errors,
loss and timeout cannot reset the send latch. Original absolute bounds are
checked at request arming and reply acceptance, in addition to the original full
frames. No property fallback or legacy-worker retry exists.

The wire uses systemd's ExtraFileDescriptors a(hs) interface as implemented in
[systemd v257](https://github.com/systemd/systemd/blob/v257/src/core/dbus-service.c).
An unsupported manager must reject it. Source/API inspection is not proof that
the local or server manager accepts these properties or preserves all required
runtime attributes; that needs its own permitted integration control.

`diagnostic_held_child_exchange.rs` reuses the unchanged ancillary parser for
atomic SCM_CREDENTIALS/SCM_PIDFD receipt and rejects rights, unknown records,
truncation, dead/wrong peers and malformed packets. The socket's SO_PEERCRED is
not used as message authentication. Readiness binds nonce, installed-entry digest,
original-context digest, manager/client hashes, invocation, creator, selected
child and original absolute BOOTTIME bound. Both sending sides latch before any
possible send. The acknowledgement is a domain-separated checksum, not a
signature or release authorization. See the versioned contract in
[COMPAT](../../../COMPAT.md#held-delivery-readiness-candidate-v1).

The child primitive has no execution or lifecycle API. Its caller must retain
the original role and creator proofs around it. It is not yet connected to the
installed child's entry; existing modes and their refusal behavior are unchanged.

## Evidence boundaries

The absent-component source contract failed first with exit 101. The initial
unlogged development compile then found an invariant-lifetime return mismatch;
its diagnostic was observed in the tool transcript, not a saved raw log. The
first retained GNU lane passed eleven new controls and stopped on a rustc 1.94
diagnostic-rendering ICE while reporting unused, not-yet-connected child code.
Its source snapshot and complete stdout/stderr are retained, without relabelling
that lane as successful. The correction marks only those staged child items as
intentionally unused outside tests and adds the explicit request-bound control.
The next GNU and MSRV lanes passed all selected controls but stopped on strict
lint: deliberate second compilation of the unchanged private ancillary leaf,
an oversized ownership enum and an oversized error result. The correction keeps
the original parser byte-for-byte unchanged with an explicitly scoped reuse
annotation, boxes the large owner variants and error, and runs both copies of
the parser's bounded/malformed-record and FD-leak controls. It does not disable
lint on the owner or replace any live check.

No benchmark, CPU/allocation measurement, original-window-fit claim, service or
clock change, namespace creation, installation or server run belongs to this
component. Frozen qualification manifest SHA256 remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

## Passing local gates

Four reviewed lanes pass the same selected 163-file source snapshot, pinned to
checked commit `e36f5dff528b062a1bef5c257ccfdacca7e42431`. Source snapshots,
exact commands, exits and raw log hashes are retained in the
[evidence packet](local-runs/bootstrap-held-delivery-20261023/audit.json).

- Windows: 68 source contracts and 24 codec/child/build controls; check,
  all-feature strict Clippy and formatting.
- GNU 1.94 and MSRV 1.88: 128 selected controls per compiler, including twelve
  new delivery/readiness controls and four controls over the two compilations
  of the unchanged ancillary parser; check and all-feature strict Clippy.
- Metadata: six Python harness/retention controls, doc-check, 188 registered
  gates, the local nonpromotable performance contract and diff checks.

The new controls include exact descriptor aliases and wire shape, original
absolute-bound refusal, manager/client/job correlation, irreversible send state,
owned-helper kernel credentials/pidfd, mismatched child context/UID, missing ACK,
send failure and a compile-only consuming-owner signature. They do not execute
the original manager delivery or the complete original owner construction.

The packet retains nine lanes, including the failed first GNU lane and both
subsequent lint refusals. Its 1,822 members plus inventory were audited as exactly
1,823 staged Git blobs before evidence commit `31388f01`. Inventory SHA256 is
`7b7e4946a42fd201e14735346838ef94528f5ff754129e647b49ebfe5926367e`.
The retention method is separately pinned to the checked commit and included in
the packet. No failed lane is promoted into the reviewed result.

The selected unsigned GNU check image is 1,639,384 bytes with SHA256
`75be0b385cace5d93db3521307218011dc5a77f4f91b8902141ae2eade09bf72`.
It was built from the checked working sources before their commit and is not a
clean signed build receipt, installed admission or activated child entry.

Commits: plan `1ff25210`; feature and necessary controls `8815e1d4`; local
runner/retention `e36f5dff`; immutable evidence `31388f01`. The report, evidence
index and article conclusions are a separate documentation change.

## Next integration gate

Implement the separate eighteen-role child entry and bounded activation
environment, preserving the old seventeen-role mode. It must prove the actual
original executable, streams and creator/guardian context around readiness, not
accept a caller's digest or root UID as enrollment. Then exercise exact manager
delivery and new-path SIGKILL/lost-ACK boundaries in a separately allowed local
service fixture. All-writer cessation, fence retention, durable completion and
server admission remain closed until their respective positive/fault gates pass.
