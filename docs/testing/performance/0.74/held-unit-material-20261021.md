# Held unit kernel material under the consumed owner

## Scope

`ProjectedHeldUnit::materialize_held_unit` consumes the existing projected owner
without accepting another config, descriptor, process, deadline or channel.
The resulting opaque `MaterializedHeldUnit` retains that owner, six new kernel
objects and an extended inventory. Its only operational method is revalidation;
failure retains the same refused owner and any completed resources. There is
no message construction, send, release, inherited-FD remapping or execution API.

The producer implements the existing HCHL0741 grammar in COMPAT. It adds no
wire/durable version, policy, authority key or qualification input. All earlier
full-frame, role, placement, guardian and retirement bodies remain unchanged.
The [local packet](local-runs/held-unit-material-20261021/README.md) distinguishes
real kernel tests from models and the still-separate original-owner live gate.

## Materialized objects

The constructor creates one nonblocking CLOEXEC stdin read pipe and immediately
closes its writer. Revalidation requires FIFO/read-only identity, zero available
bytes and exact HUP without readable data: this input is EOF, not a caller-fed
stream. It is distinct from both original stdout/stderr pipes.

A new CLOEXEC/nonblocking AF_UNIX SEQPACKET socketpair retains both release
endpoints. Both have SO_PASSCRED enabled and SO_PEERCRED matching the original
creator's PID and effective UID/GID. Fresh non-consuming peeks require empty
queues; data, a zero-length packet, disconnect or changed flags refuses. No
Ready or Release message is generated or sent by production materialization.

The config comes only from the original bundle's surface selected by the
retained cell intent. A bounded positional copy checks original device/inode/
length, complete metadata stamp, signed digest and unchanged stamp after read.
It preserves the original file offset. A fresh memfd receives those exact bytes,
mode 0400 and the exact SEAL/GROW/SHRINK/WRITE seals; the writable creator alias
closes before publication. The retained handle is read-only and CLOEXEC.

Creator pidfd construction first requires the same original process, thread,
real/effective credentials and live absolute deadline. It opens only the current
creator PID, not a supplied or reusable observed peer PID. Each inspection checks
bounded procfs fdinfo, the exact creator PID, nonterminal poll state and the
retained descriptor/object/flag identity. No process is created or signalled.

The read-only sealed 256-byte bootstrap binds the original minimum deadline,
creator credentials, both retained Armed root guardian PIDs, a nonzero kernel
random token, worker/config hashes, the original worker object, the new sealed
config object, three stdio identities and the launcher-side release endpoint.
Reserved bytes stay zero. Fresh checks rebuild and compare every byte; the
token and bootstrap are not exported. The existing launcher decoder and fixed
0–10 role grammar are unchanged.

## Expanded inventory and refusal

The extended snapshot covers 20 distinct descriptor numbers: the original ten
roles, two addressed-message aliases, two original custody pidfds and six new
objects (stdin, both release endpoints, sealed config, bootstrap, creator pidfd).
It compares object metadata, mode/credentials, descriptor/status flags and
regular-file positions. Negative or duplicate slots, aliasing a new FD into an
old slot, replacement objects and position/flag drift refuse.

This is a specified role inventory, not a census of every descriptor in the
supervisor or proof that every external writer has ceased. Retained role/bundle
files and root guardians keep their existing complete live validators; their
identities, root, boot, state, manager correlation, origins, bounds and document
digests also remain in the extended context. No existing inventory is replaced
or permitted to ignore drift.

Material creation and each later inspection follow the same bracket: complete
projected-owner revalidation, create-only-if-uninitialized, fresh extended
inspection, complete projected-owner revalidation, fresh extended inspection
and equality. Both projected-owner calls still execute their unchanged full
holder rounds. Revalidation compares against the original stored inventory and
never recreates a lost object or refreshes an origin. The original 300/60/10
windows remain; fresh kernel time reads only reject an expired bound.
After the last FD stamp, another fresh original-owner/deadline observation
closes the new census before returning its snapshot.

First failure retires the original owner and is sticky. Successful objects
remain owned until the holder/failure is dropped. Drop retires the old owner
before closing the new objects, then drops the old holder in its existing order.
Partial local creation uses RAII to close intermediate descriptors; no new
resource has been sent or granted to a writer. No guardian is disarmed and no
completion, safe-unlock or retry capability is added.

## Local verification boundary

The selected new controls exercise real local pipe/socket/memfd/pidfd mechanics,
the exact bootstrap layout and copying, rejected writes, altered flags/offsets/
objects, queued data and zero-length packets, disconnect, expired synthetic
bounds, inventory substitutions, closure and a late partial-construction leak
oracle. A fixed seed `0x0740fd61` drives 256 bootstrap byte substitutions.
Separate model controls cover opening/effect/inspection/closing failures,
retained/closing context drift and sticky refusal. The API contract compiles
the actual consuming method, not an alternate transport.

Kernel tests use synthetic role/stream/guardian expectations; they do not claim
that those claims passed the authenticated original frame. No new ignored,
privileged or whole original-owner fixture is invoked. Source canaries restore
the exact two additive hooks and reconstruct historical full runtime/fixture
bodies. Closed-API checks forbid send, exec, remapping and FD extraction.

The initial local cohort passed its mechanics tests, but review found the new
twenty-FD census after the final fresh deadline sample. That candidate is
retained and not accepted as complete. The final candidate adds the closing
original-bound check and its mandatory source canary. A separate complete
cohort binds the corrected source; exact reconstruction proves these are the
only source differences, without extending a deadline or removing an old check.

The final GNU 1.94 and MSRV 1.88 cohorts each pass 82 selected tests: twelve new
materialization controls, ten projection, ten consuming-holder, twelve reply
and thirty-eight synthetic held-build controls. Thirteen host source-contract
tests pass, including exact reconstruction of 462 historical runtime/fixture
files and the new mandatory closing-census check. Affected-package all-target
checks and strict all-feature clippy pass on both Linux compilers and Windows;
format, doc-check, all 186 registered gates, local non-promotable performance
contract and diff checks pass. No full release qualification is claimed.

Next run a separately retained whole original-owner projection/materialization
control inside the original absolute windows. This remains a timing/composition
gate: isolated kernel success does not establish that the additional full
rounds fit the original preparation budget. Preserve any first refusal without
retry-to-pass or extending a window. After that, durable single-delivery
uncertainty and authenticated manager/process/barrier binding remain before
active collection, all-writer cessation, completion/cells 2–4 or joined
SIGKILL/lost-ACK/reconnect admission. Server enrollment/rehearsal is separate.
No clocks, services, host settings, installation, supervisor or server are
changed; no product workload or performance measurement is run.
