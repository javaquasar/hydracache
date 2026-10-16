# Durable collector Reserved under the original request owner

Baseline `13dac632d91bb029821b9f3a2678e13694222130`. Consume the observed
reservation into a distinct owned publication, never into the legacy regular-file
publisher. Keep the original controller/OIDC/build/kernel/worker/authority and
common fence. No second journal opener, fence gap, recovered adoption or caller
state/clock/peer/path/descriptor input is allowed.

Add `diagnostic-collector-reserved-074-v1` in the exclusively created fixed
`diagnostic-collector-reserved-074.json`, beside the original controller attempt.
Canonical LF JSON, at most 16384 bytes, contains the frozen observed initial
Reserved state, the separately signed collector envelope and SHA256 of the
complete original controller-attempt bytes. Register the additive format in
COMPAT. A strict bounded paired byte inspector checks schema/canonical grammar,
original request/hash/identity/window, worker policy/epoch and state/unit hashes;
it verifies neither signatures nor runtime acknowledgement and cannot open a
publication guard. Unknown versions, partial/mixed pairs and extra entries refuse.

The old AttemptStorage one-document inspection and legacy transfer predicates
stay unchanged. A private child storage engine consumes that original storage
and retains its descriptor/fence, checking an exact one-to-two-document inventory
and both original objects/ancestry around operations. Create with exclusive,
no-follow CLOEXEC open, write in two halves, seal 0400, sync file and directory.
Revalidate the same live original authority, signed requirements, request and
kernel clock at every boundary. Acknowledge only after all checks/syncs; later
revalidation takes no replacements. Failure retains the original owner/fence
and uncertain context, cross-refuses without repair/unlink or another I/O retry.

Checkpoints in fixed order: BeforeCreate, Created, CreateDirectorySynced,
PrefixWritten, BytesWritten, Sealed, FileSynced, DirectorySynced, Acknowledging.
Expose checkpoints only to private tests. Cover logical failures at all nine
stages (seed 0x0740c09a), valid signed substitutions, old-format refusal, namespace/
original-object/ancestry/source drift, late sticky refusal, independent owners
and consuming/readonly API compile-fail checks. Record baseline and compiler red
before implementation. Keep rejected bytes, mode and exact lock lifetime.

Add a registered ignored/manual nonroot Linux subprocess proof using the original
kernel sender chain. Seed 0x0740c09b shuffles all nine stages plus AfterReturn;
SIGKILL only the owned unreaped child through its pinned pidfd, with one positive
control. Inspect each retained case twice in fresh read-only processes. All
stages, including BeforeCreate, refuse fresh reservation because the original
request is already retained. Run the ten-kill/one-control/22-inspection matrix
three times; prove unchanged retained origin/authority and lock release, without
reconstructing acknowledgement. This is not power-loss/reboot proof.

Run current/MSRV targeted original-chain, signed-byte and compile-fail regressions,
check/strict Clippy for affected packages, format/static/registry/doc gates and
closed ship admission. Amend historical source guards only for exact module/
registry additions; do not weaken existing predicates or numerical budgets.
Retain source-pinned evidence and explain the durable-versus-authenticated-start
boundary in the article/checklists. Commit small logical slices; do not push.

This closes only initial collector Reserved publication. Starting, root ancestry/
outputs, authenticated original start, watchdog/all-writer cessation, completion,
later cells and real enrollment stay open. No product code, frozen qualification,
server, workflow or expensive workload changes. Historical Kernel(Invalid)
remains unexplained. Production and release admission stay closed.
