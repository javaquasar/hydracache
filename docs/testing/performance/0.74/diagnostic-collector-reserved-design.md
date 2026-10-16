# Initial collector Reserved publication

The [publication plan](diagnostic-collector-reserved-plan-20261016.md) connects
the observed initial state to durable retention under the same original request,
authority and common fence. The returned guard acknowledges initial Reserved
only. It cannot prepare output, start a process, publish Starting/completion or
advance later cells. Production trust and host enrollment remain separate gates.

## Record and ownership

OriginalObservedReservation::publish_reserved consumes the observed owner and
accepts signed collector bytes only. Before transfer it uses the existing
original preflight to check the signature, frozen state, request/nonce/controller,
complete window and original kernel/worker authority. Preparation errors retain
that observed owner in the returned failure; they do not release/reacquire its
fence or leave it eligible for another candidate.

Successful preflight moves the original request storage into a private child
engine. It retains the request file/bytes/stamp and original fence, but has its
own exact inventory for the old request plus the new record. The legacy
AttemptStorage one-file inspector and regular-file publication transfer bodies
remain unchanged. No old request guard is reconstructed or used to inspect a
two-document namespace.

The separate COMPAT-registered diagnostic-collector-reserved-074-v1 document is
canonical LF JSON, at most 16384 bytes, exclusively created as
diagnostic-collector-reserved-074.json in the original fixed intent leaf. It
contains the complete frozen initial state, separately signed collector envelope
and SHA256 of the complete original controller-attempt bytes. The latter binds
the original request, OIDC token digest, build/configuration pins and active
authority generation without retaining the raw token.

validate_retained_collector_reserved checks two bounded canonical byte documents,
their schemas, original request/state/window/controller/policy/epoch relationship,
initial equal clocks, signed-envelope grammar and state/unit digests. It does not
verify a signature or prove that any clock was observed. For example, a
syntactically valid all-zero signature can pass that inspector. Only the original
runtime owner supplies authentication and observed-state provenance; no decoded
record can reconstruct its acknowledgement or become execution authority.

## Publication and failure

The new storage engine checks original ancestry, same common fence, original
request object, new owned record when present and exact inventory around each
checkpoint. Creation uses the existing descriptor-relative exclusive/no-follow/
CLOEXEC helper. Nine ordered checkpoints cover BeforeCreate, Created,
CreateDirectorySynced, PrefixWritten, BytesWritten, Sealed, FileSynced,
DirectorySynced and Acknowledging. Bytes are written in two halves; the original
created file is sealed 0400, then synced with its directory. The runtime
acknowledgement flag is set only after final checks and sync.

Each boundary rechecks the same live controller/OIDC/build/worker/authority,
original request bytes, collector signature and original receiver clock. The
initial state and request window stay fixed. Clock-order observations neither
renew the original packet hold nor count as new controller progress. The
returned guard brackets those checks with the two-document storage checks and
accepts no replacement parameters.

Successful revalidation establishes retained-record/owner consistency, not an
execution lease or heartbeat permit. Starting must compare fresh kernel time
with the fixed lease/progress deadlines and require actual authenticated
controller progress; it cannot infer renewed liveness from this receipt.

Any failure latches all original owners and retains the original chain/fence
plus partial or complete record in the failure object until drop. It neither
unlinks nor repairs anything and appends no new durable authority refusal.
After refusal revalidation returns before new I/O. A 0400 file is not proof that
all writable descriptors have been revoked; this is journal retention, not
output writer-cessation/completion evidence.

## Process loss and remaining lifecycle

Even BeforeCreate already has the durable original controller attempt. Every
fresh read-only inspector must therefore refuse reservation at every new crash
checkpoint, including AfterReturn. Its ability to parse a complete record cannot
recover the lost runtime acknowledgement. The separate manual nonroot Linux
proof uses exact owned child pidfds, SIGKILL, an uninterrupted control and two
fresh inspections of each retained origin/authority/context snapshot. It checks
process loss, not power loss, host reboot or rollback resistance.

Starting needs its own consuming protocol and crash boundaries. Root ancestry,
exclusive collector output creation, authenticated original executable/streams/
process/manager/cgroup start, actual controller progress, autonomous watchdog and
all-writer cessation remain required before completion and later cells. Reads
of this guard cannot substitute for a newly authenticated heartbeat. Real trust,
installation and nonproduct host rehearsal are still pending. Frozen product
qualification and numerical claims are unchanged; Kernel(Invalid) remains
unexplained.
