# Namespace checked credential opening and reads

## Local scope and baseline

Baseline `b087ae92825cc3b3d1fbf64742e90b28b966ae26`. Add a separate opt-in
Linux reader accepting only the original `ProcessRead` and existing numeric
credential assertions. Preserve the numeric-only API and account binder. No
already opened credential reader, replacement PID, path, FD or namespace token
can be enrolled. Retain baseline, missing-API check, tests, implementation and
repeated local results separately.

## Opening context and sequence

Linux [task_state](https://github.com/torvalds/linux/blob/v6.6/fs/proc/array.c)
projects UID, GID and supplementary groups through `seq_user_ns`. The
[seq_user_ns helper](https://github.com/torvalds/linux/blob/v6.6/include/linux/seq_file.h)
uses the file's retained credential user namespace. Consequently a namespace
guard created after opening status cannot certify that opening context.

Pin the original process and current thread's shared user namespace first.
Then require namespace, open original status, namespace, credentials, namespace
in that order. Open status once relative to the original proc directory with
the existing bounded no-follow document reader. Do not read credentials before
the post-open namespace check. Subsequent calls require namespace, credentials,
namespace; each component retains its original generation/document/namespace
checks. First failure stops observation and refuses both privately owned guards.
After refusal there is no further IO, refresh or extraction of either guard.

This is a sequential opening/read bracket, not a kernel-attested file-credential
namespace inspection, atomic snapshot or continuous monitoring. Same-thread
synchronous opening narrows the context gap but cannot certify unobserved
transitions between observations, trusted host/initial namespace, all threads,
NSS provider/configuration or authenticated original start. No namespace join,
account mutation, helper/IPC route or production admission is added.

## Tests and gates

Test exact constructor and later-read order, every first failing step, sticky
refusal with no later callback, and seeded failure-stage/error variations.
Real owned non-product no-new-privileges cat helpers exercise repeated reads,
wrong assertions, unhardened credentials, original exit, retained status FD
and named document substitutions with restoration, namespace FD substitution,
and independent concurrent readers. Test seams stay private and test-only.
No test creates or joins a foreign user namespace; modeled drift is labeled.

Run scoped supervisor tests, Linux/Windows check and clippy, formatting, root
performance/evidence/governance tests, documentation/link/book checks, and
expected-red require-ship admission. Retain exact raw captures and SHA-256
verification. Do not change the frozen qualification manifest, thresholds or
existing packets. No host install, workload, qualification or performance claim.
