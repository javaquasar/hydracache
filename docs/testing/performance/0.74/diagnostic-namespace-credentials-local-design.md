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

## Local results and remaining boundary

The [packet](local-runs/diagnostic-namespace-credentials-20261010/manifest.json)
retains the 230-pass baseline, missing-API capture, ten-test first green and
242-test final Linux suite with one pre-existing ignored. Review adds the
pre-refused component and corrupted private opener-state cases; all twelve new
tests pass three repetitions. Seed `0x74f2026` varies 256 modeled first-error
and stage cases per repetition. Constructor-stage injection is modeled; real
kernel cases use owned NNP cat helpers, the original-process test pin and current
reading thread. No foreign user namespace is created or joined.

Status FD and named-document substitutions, namespace FD substitution and
original process exit all propagate refusal to both privately owned components.
Restoration cannot refresh them. Independent concurrent readers and the existing
numeric API remain usable after another reader refuses. Wrong numeric policy
and a helper without NNP refuse during construction. Private-state corruption
refuses without reopening the retained status document.

Windows passes 28 portable tests and executes no Linux cases. Root contracts,
evidence and governance pass 126 tests. Scoped formatting/check/clippy and
documentation gates pass; require-ship stays expected red with native exit 1.
The API-red command wrapper initially lost its exit variable, then incorrectly
expected 1 where Cargo returned 101; both raw captures and the wrapper failures
are recorded separately from implementation tests. The qualification digest is
unchanged. No full workspace milestone gate or numerical measurement ran.

This step implements the sequential context bracket only. Trusted NSS provider
and host/initial namespace policy, all-thread proof, root-owned fenced output
preparation, authenticated original start and watchdog recovery still precede
production composition and a positive lifecycle rehearsal.

The subsequent [signed policy credential binding](diagnostic-policy-credentials-local-design.md)
compares this retained reader's assertions with the original signed local mapping
and brackets revalidation with observed context/account reads. Private boolean
adapters expose no projection or replacement policy; original opening/parser
behavior is unchanged. This read-time consistency does not retroactively attest
status opening under the signed host/mount context.
