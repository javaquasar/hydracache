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
