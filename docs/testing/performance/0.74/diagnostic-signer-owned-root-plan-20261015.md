# Normal-build fixed GNU root preparation

Baseline `81c327d1`. Move only fixed root materialization into a binary-private
normal-build module. Do not promote the independent cfg(test) rooted launcher or
change the ordinary owner, isolation policy, GNU parser or pinned helpers.

The fixed `prepare` CLI accepts no paths, commands, descriptors or runtime inputs.
Require an empty environment, non-root equal real/effective IDs and a sole main
thread. Retain four original read-only CLOEXEC source descriptors: self and the
fixed loader/libc/libgcc paths. Runtime path resolution deliberately follows the
distribution's fixed symlinks; numeric root ownership and non-writability are
observations, not authenticated ancestry, initial-namespace provenance or custody.
Keep the existing 8 MiB/image bound, stable complete metadata and GNU declaration
graph validation. Debug executables exceeding that limit must refuse. Tests may
strip a private executable copy, never the Cargo artifact, without changing the
limit or release build profile.

Create one random 128-bit name under retained `/tmp`, with bounded getrandom
interrupt handling and no collision retry/adoption. Use descriptor-relative
exclusive creation: root/lib/x86_64-linux-gnu and root/lib64; signer, loader,
libc and libgcc only. Close every writable file descriptor before sealing files
and directories to 0555. Retain original directory/file descriptors, stamps and
hashes. Rechecks must reject missing, substituted, extra, hardlinked, writable or
changed objects without recapturing them. Source descriptors stay retained too.

Cleanup is not recursive: preflight every known name and exact directory listing
against originals before chmod/unlink; remove only four known files, three known
subdirectories and the original fresh root name. Refuse foreign substitutions;
never delete a preexisting collision. Track partial construction for bounded
cleanup. This is a cooperative local process scope, not protection against a
hostile same-UID/privileged actor racing validation and unlink, durable independent
custody, writer revocation or mapped-byte attestation. Explicit cleanup failure
must prevent success output; Drop is best effort only.

Before implementation capture GNU/root-fixture baseline and the absent normal
target failure. Cover synthetic fixed graphs, original descriptor retention,
partial construction, collisions, content/mode/link/source drift, equal-byte
substitution, symlinks/extra entries, safe refusal and independent builders. Add
actual normal-process empty-environment preparation, bounded private stripped
image, invalid CLI/environment/image and four concurrent process checks. Run
current/MSRV focused tests, all-target check/strict lint, formatting, registry,
root/doc/local-contract checks and expected ship refusal. Preserve raw results
and failures; distinguish synthetic graph tests from real runtime observations.

No child launch/chroot/namespace changes this slice; no credentials, signing,
server/GitHub operations, performance workloads, qualification, completion or
later cells. All authorization, release and production admission remain false.
Next slice joins original child launch with selected root/runtime and independently
checks post-exec kernel mappings, without weakening existing isolation checks.

Observed during implementation: the local Rust1.88 normal executable declares
libm in addition to the four-role profile. Preserve the initial 1/2 process-test
failure and readelf output. Do not expand the approved Rust1.94 graph. MSRV must
compile and pass synthetic materialization, while its independently identified
exact extra-libm normal profile must refuse. Unknown dependency profiles still
fail the test. Rust1.94 normal preparation must remain an actual success; an
extra-libm refusal is not a successful root materialization or MSRV runtime claim.

The broad old-root baseline failed 78/1 in an unchanged constructor-loss seeded
observer before new code was compiled. Retain it as an unresolved regression
observation, not a fixed issue or a reason to weaken checks. Local focused gates
for this isolated module do not constitute an all-regressions/server green gate.
