# Observed Linux worker policy context

## Scope and authority

Baseline is `2a27e34ebf8c13ede7b8a2cdac146edfc4fbedee`. This opt-in,
read-only slice compares actual fixed machine/boot files and typed reading-thread
user/mount namespace objects with the already verified signed worker policy.
External issuer, body digest and epoch remain independently supplied. Observation
cannot choose or refresh these pins and is not initial-host namespace authority.
Existing account-file, NSS, numeric credential, receipt, helper and start APIs
remain unchanged. Production preparation and qualification remain closed.

## Fixed observations and original objects

Retain read-only descriptors for `/`, root-owned non-writable `/etc`, its single
linked regular `machine-id`, procfs `/proc/sys/kernel/random/boot_id`, and current
`/proc/thread-self/ns/user` and `mnt`. Walk literal components with openat,
NOFOLLOW, CLOEXEC and NONBLOCK. Follow only the three deliberate proc magic
links thread-self, user and mnt beneath validated procfs. Validate both namespace
handles as nsfs with NS_GET_NSTYPE equal to their distinct expected types.

Machine ID requires exactly 32 lowercase hex digits and LF; boot ID exactly
the 36-character canonical lowercase UUID and LF. Compare directly to original
policy values; all-zero IDs were already refused by the policy verifier.
Each document uses one positional read into its exact length plus one byte:
short, interrupted, overrun or metadata-changing reads refuse without retry.
Procfs documents need not advertise their content length. Files retain object,
owner, mode, link and size/time metadata; directories retain object/owner/mode,
not maintenance timestamps. Reopen fixed names and compare original objects
before and after observations. No path, FD or context extractor is public.

Open identity documents inside a namespace bracket. Revalidation first checks
original policy/trust/bytes/caller assertion, then twice performs namespace,
machine/boot documents, namespace. Check the original reading thread ID and make
the runtime guard neither Send nor Sync; transferring a reader to a thread with
an equal namespace is not admitted. All failures latch the borrowed original
policy, including construction failure and after wrapper drop. Refused policy
precedes any new IO. Successful drop leaves the policy usable.

## Proof limits and verification

This is sequential current-thread consistency, not an atomic cross-file or
continuous snapshot, physical-host attestation, proof of the initial namespace,
all-thread enrollment, retained file-opener credentials, writer revocation or
regular-file IO deadline. Root ownership is evaluated inside the observed user
namespace. Device/inode pins are meaningful only in the externally approved
boot context. A restored object indistinguishable at every observation is outside
the proof. Durable external epoch/refusal management remains separate.

Tests use synthetic signer keys and actual unprivileged local Linux observations;
they never enroll a real issuer or modify machine/boot files or namespaces.
Private fixtures exercise file safety, exact/partial reads, wrong namespace
types, descriptor substitution/restoration, thread mismatch, trust/policy
precedence, independent readers and seeded valid re-signed context mutations.
Record baseline and missing API refusal before implementation. Run focused Linux
and Windows regressions, affected check/lint, root evidence/governance contracts,
format, doc/local performance contract, links/book and expected-red require-ship.
Retain immutable raw logs and frozen qualification input identity. No workload,
server mutation, performance claim or production capability follows this guard.

## Implemented guard and retained results

Implementation `f861e460137d6a35ba2dc519310d4796f79aefdd` adds the Linux
`local_context` child with a privately constructed borrowed `WorkerContextRead`.
Only refusal status and revalidation are exported. Existing policy schemas and
assertion APIs stay intact, as do account-file, NSS, numeric, namespace and
production paths. Opening, named-object comparison, typed namespace checks and
sticky policy refusal follow the preregistered sequence.

The [packet](local-runs/diagnostic-worker-context-20261011/manifest.json) retains
the 43-pass Windows baseline and missing-API E0432 before implementation.
First green passes five integration tests. Final Linux passes 295 (230 library,
29 artifacts, 2 manager, 3 account, 5 context, 13 files and 13 policy), with one
existing system-bus test ignored. Windows passes 43 portable cases; root contracts,
evidence and governance pass 130. The new root contract also passes on Linux.
All five integration and seven unit cases pass three further repetitions.
Seed `0x7542026` checks 64 valid re-signed namespace mutations per run: byte
verification succeeds under synthetic pins, actual context observation refuses.

Wrong namespace types, original descriptor/thread substitution, short/overrun/
interrupted reads, mid-read growth, unsafe owned fixtures and named-object
replacement refuse. Original restoration never clears a latched policy. A
compile-time test checks both negative Send and Sync properties. Scoped package
check/strict lint, format and documentation gates pass; require-ship remains
expected native exit 1. No test mutates real identity documents or namespaces.
The frozen qualification digest remains unchanged. These are local safety gates,
not full-workspace qualification, real issuer enrollment or performance data.

Next: open account files inside this original context and compose their mapping
with namespace-checked kernel credentials, preserving all original refusals.
Dropping independent readers in sequence is not that composition. Durable
external authority and authenticated lifecycle activation remain separate.
