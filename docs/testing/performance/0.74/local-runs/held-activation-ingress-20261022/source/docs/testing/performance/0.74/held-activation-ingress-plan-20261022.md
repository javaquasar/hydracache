# Strict activation ingress before the unchanged held launcher

Baseline is aa58bad7. Preserve all old core launcher, original-owner frame,
retirement, qualification and absolute 300/60/10 origins/windows. No manager
calls, unit/service mutations, clock changes, server runs or product workload.

Add an explicit `--systemd-activation-v1` entry to the same launcher target.
Only this entry may accept activation metadata. Require exactly LISTEN_PID equal
to the current process's canonical decimal PID, LISTEN_FDS equal to 8 and
LISTEN_FDNAMES equal to
`release:worker:config:creator:root-a:root-b:alarm:bootstrap`. Reject missing,
duplicate, unknown, noncanonical or malformed names/values before mutation.
Require a single current thread, remove only those three validated variables,
verify an empty environment and call the unchanged old core. The old no-argument
entry still rejects any environment; no automatic fallback or parser retry.

This is an ingress protocol, not manager authentication or descriptor-role
identity. All old descriptor/content/creator/guardian/deadline/release checks
remain responsible for those facts. Unknown manager-default variables must
refuse rather than being silently cleared. Actual unit admission will need to
prove its complete pre-exec environment; do not infer that from post-exec cleanup.

Initial executable addressing remains independently gated. In upstream systemd
v257 exec-invoke.c, find_executable_full precedes shift-related final FD layout.
Therefore `/proc/self/fd/11` cannot be assumed to name a supplied launcher at
ExecStart lookup. A creator PID pathname is not a stable retained-object
capability across owner death/PID reuse. Do not implement either as a fallback.
Choosing a retained read-only mount alias or separately enrolled bootstrap
requires explicit architecture selection before that dependent implementation.

First add a red current-source contract, then parser tests and local synthetic
process controls that stop at the old core's missing-descriptor refusal. Run
GNU 1.94/MSRV 1.88 binary tests, the existing held_launcher target without its
ignored fault matrix, selected source contracts, check, strict clippy, formatting
and documentation/contracts. Keep first outcomes and do not repeat any original
or measured attempt. Register the new ingress format without altering v1 core.
