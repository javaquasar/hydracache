# Durable diagnostic attempt before Reserved publication

This P1 slice retains the original signed initial attempt before publishing any
Reserved state. Baseline is `277f774a4fabad2b3e76fa3f907554f51cc45d22`. It is an isolated local
primitive, not fixed lifecycle completion or a production entry. The governing
[intermediate plan](diagnostic-server-readiness-intermediate-plan-20261012.md)
keeps peer/time, execution and host enrollment closed.

## Admission boundary

Before the first durable mutation, a process crash leaves no accepted attempt.
No local journal can prove a nonce consumed before anything reaches storage.
The API acknowledges an attempt only after exclusive creation, complete bytes,
sealing, file/directory sync and final original-object/authority checks. Any
retained empty, partial or complete object blocks new empty-fence reservation.
Death after successful return loses the runtime acknowledgement; surviving
bytes do not authorize replay or reconstruction.

The sole new filename is `diagnostic-attempt-074.json` in the already bound
fixed intent leaf. It reuses canonical LF-terminated
`diagnostic-retained-intent-074-v1` bytes and the existing 16384-byte budget:
initial Reserved state, exact signed intent, asserted peer UID/groups and null
previous-record hash. The signed intent contains request/nonce, controller,
worker policy epoch and state/unit digests. The name distinguishes early attempt
retention from cell-one consumption. There is no commit marker, new schema,
persisted boolean acknowledgement or claim of globally consumed nonce.

## Original ownership and refusal

Only the original fixed intent and owned original empty fence can enter.
Require the complete originally bound revision-one Reserved state with zero
completed cells before any write; a mismatching unbound assertion does not
revoke healthy inputs. Revalidate the original intent/fence before mutation.
Use descriptor-relative O_EXCL/no-follow creation with mode 0600; retain the
created descriptor and expected partial bytes, seal only that owned file to
0400, and check exact inode/owner/group/mode/link/size/mtime/ctime and bytes.
The leaf inventory must contain exactly the objects actually owned by this
attempt, never an allowlist permitting adoption.

The returned opaque guard owns the original fence and exclusive intent borrow.
Failure retains the original lock until drop, latches both chains and publishes
Policy refusal only through the independently safe original authority journal.
Intent/preflight errors use their existing bridge once. Refusal uncertainty
preserves the first error. No path/descriptor/state export, Clone, fixture
conversion, repair, cleanup, reopen, retry or start is added.

## Preregistered tests

Baseline the existing initial publication tests. Retain an absent-entry red test
before implementing the new API. Cover all eight boundaries: BeforeCreate,
Created, PrefixWritten, BytesWritten, Sealed, FileSynced, DirectorySynced and
Acknowledging. Test canonical healthy retention, unbound mismatch, all logical
interruptions, late target/context, original replacement/links/byte/mode drift,
expiry/signature/peer/state drift, safe and unsafe refusal, sticky errors and
independent concurrent original chains.

Extend the owned nonroot process harness with an attempt-only phase. A manual
ignored gate performs SIGKILL at every boundary and after API return, plus an
uninterrupted control. Seed `0x074d` deterministically permutes all cases.
Confirm lock contention/release, exact owned helper termination, unchanged
retained bytes/metadata and two fresh read-only processes per case. BeforeCreate
must observe empty, not consumed. Later cases must refuse new reservation.
Run three focused functional and process-loss repeats; format, all-target
check, warnings-denied Clippy, MSRV, targeted regression and documentation checks.
Retain exact source and raw logs separately. No actual host power loss or reboot
is claimed.

## Compatibility and remaining P1 work

Old empty-fence readers and campaign admission already reject any fixed-leaf
context, including this new filename. Existing state publisher and cell-one
consumer are unchanged and cannot adopt the retained attempt. There is no
downgrade with unresolved context, automatic migration or implicit deletion.
The exact original attempt must later be consumed by an explicitly owned
progression protocol; that integration and fixed cells two through four remain
open. Removing retained context or rolling back an entire host is outside this
local proof. Production authority enrollment and a reviewed resolution process
are prerequisites, not inferred from a readable file.
