# Fixed controller principals and original kernel policy binding

Baseline is `cbbae6499c6eccc902df99a8d1bc103c013a4f85`. The worker/kernel
guard already retains original policy/config/journal and Linux context. This P2
step adds an independent source for controller principals; it does not choose
real actor/account values, install configuration or admit execution.

## Fixed configuration

Add the separate canonical LF JSON schema
`diagnostic-controller-trust-input-074-v1` at the single fixed path
`/etc/hydracache-perf/controller-trust-074.json`, with the existing 2048-byte
root-file budget. Its fields are schema_version, repository_id,
controller_key_hex, actor_ids, client_uids and client_gid. The key/repository must
match the separately reviewed controller pins. Require nonempty strictly sorted
unique allowlists of at most 32 entries; actor IDs are nonzero and client IDs/GID
are nonroot, not the reserved maximum UID. No default, envelope field, environment,
key selfcheck or worker-policy value selects these principals. Unknown, duplicate,
future, missing/null and noncanonical documents refuse.

Reuse the existing private root/directory/file metadata and bounded byte checks,
but retain an independent original root/ancestry/document for this distinct path.
The public constructor takes no path, FD, key or principal argument and requires
preliminary effective UID/GID zero. There is no writer, provisioning, reload,
permission repair, serializer or trust extractor. Scalar root ownership is not
physical initial-host or file-opener enrollment.

## Original signed request observation

Authenticate only through the existing original KernelWorkerPolicyInput packet:
worker revalidation, controller config check, kernel signature/peer/time check,
controller config check, worker revalidation. The controller input supplies its
retained fixed principal policy, never a caller-selected allowlist. Signature and
boot/freshness checks remain the existing diagnostic request/domain/preset.

Return a read-only original-thread SignedControllerInput retaining mutable
borrows of controller source and the same worker/kernel guard, plus original
request data/hash. No request/policy/FD extractor, clone, replacement revalidation,
fixture conversion, replay consumption, start or completion method exists.
Any construction/revalidation error latches controller and every original
worker/config/journal/packet runtime input. Healthy drop leaves owners healthy.
Reauthentication must produce the same original request and never renews its
exclusive expiry or the existing 60-second packet hold.

A signature authenticates the selected signer's statement. GitHub run_id,
run_attempt, actor_id and build-provenance hash are not independently verified
GitHub/build provenance by this guard. Reading the same packet is not nonce
consumption. Next join independently verified provenance and original durable
replay/early-attempt acknowledgement under the common host fence; only then
connect authenticated collector start, autonomous watchdog and writer cessation.
Real worker policy/epoch and explicit authority enrollment remain required.

## Preregistered local checks

Retain focused worker/kernel, kernel and reviewed-trust baseline; first capture
the missing public API compile refusal. Positive unit fixtures use synthetic
worker/controller keys, actual local Linux context and kernel messages, with
owned private nonroot config/journal roots. Never sign with a real private key or
create actual controller configuration. Exercise strict schema/principal/key/
budget checks, fixed constructor nonroot refusal, signed request roundtrip and
healthy drop, wrong actor/UID/GID/key/signature/time/boot, source name/content/mode
drift, original worker/packet refusal and sticky cross-refusal.

Use seed `0x07532074`, 32 signed disallowed-actor mutations and four independent
bindings. Add compile-fail extraction/start/thread/borrow-refresh constraints.
Run three focused repeats, affected regression, current/MSRV checks, strict
current lint, doctests, scoped format, root contract/evidence and documentation/
non-promotable performance gates. Reuse the registered Linux worker/kernel API
target; no new manual process-loss proof is claimed for this read-only step.
Register the new configuration in COMPAT; old wire/durable formats, frozen
qualification inputs, keys, product crates and provisioning stay unchanged.
