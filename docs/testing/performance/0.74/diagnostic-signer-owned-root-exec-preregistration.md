# Ordinary selected root execution probe

Base source: `73b5ccd515d8f47d0295a0f6b87628fed937c0e2`. This local launch-mechanics
step connects the normal private four-image materializer to an original fork,
same-ID user namespace, selected root, first exec and loaded kernel VMA metadata.
It deliberately stops before the procfs-dependent restriction installer and any
handoff. No credentials, signing, product workload, host changes or remote runs.

## Hypothesis and unchanged bounds

A fixed empty-environment `root-exec` command can execute the copied original
self using `execveat`, observe the two initial images and four loaded images,
then kill and reap that exact child and drain its private pipe before cleanup.
The existing `prepare` behavior, four-image GNU profile, 8 MiB image bound,
five-second wait bound, old fixtures and isolation policy remain unchanged.
Rust 1.88's independently observed extra libm remains an explicit refusal.

## Implementation sequence

1. Record the existing preparation baseline and a failing new fixed-mode test.
2. Introduce only binary-private owned launch state. Arm PDEATHSIG before the
   initial ptrace stop; install TRACEEXEC and EXITKILL before clearing it for
   the user namespace transition. Never adopt an external PID or supplied FD.
3. Retain the original process directory, pidfd, root and executable. Map only
   the parent's nonzero UID/GID, change only the child root/cwd, remove capability
   sets, enable no_new_privs, then observe an exact rooted stop and first exec.
4. Check root/cwd, namespace, credentials, capabilities, executable, FD inventory,
   environment and bounded maps through the retained original process directory.
   Resume only once to a fixed nonauthorizing child marker and SIGSTOP. Check
   loaded mappings, then kill without resuming the stopped child.
5. Prove original-child terminal wait/reap and private stream EOF before removing
   the root. Preserve the root if child/stream quiescence cannot be established.
   Refusal is sticky; cleanup never recursively deletes or adopts replacements.

## Tests and acceptance

Add parser mutation tests, wrong-phase and cleanup coverage, actual normal-build
rooted exec tests, repeated independent runs, environment/argument/descriptor and
SIGCHLD refusals. Run targeted preparation and old launcher/first-exec regressions,
current and MSRV checks, strict lint, formatting and local static contracts.
Retain failures and distinguish development checks from final-source reruns.

The internal child mode is a harmless fixed diagnostic, not authentication. Its
private fixed marker is version 1 and must not become authority. VMA checks cover
kernel metadata, not loaded bytes, relocations, hostile same-UID custody, all
descendants or P2/P3 writer revocation. Original-start, policy composition,
completion, later cells, server qualification and ship gates remain closed.
