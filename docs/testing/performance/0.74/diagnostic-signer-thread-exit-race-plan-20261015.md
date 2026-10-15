# Attribute the parallel tracer thread negative control failure

Baseline `7eca2350`. The previous owner-loss packet retains two default-parallel
13/1 failures in the unchanged 14-test creator/thread-exit group and a serial
14-test success. A fresh parallel baseline may pass; neither retry nor serial
execution repairs this unassigned failure.

First add test-only bounded refusal context to distinguish creator disappearance,
original TGID liveness, original child liveness and ptrace detach. Keep exact
SIGKILL/reap/ECHILD predicates, six positive cuts, healthy controls, original
descriptors and existing 5s/30s bounds. Do not change the normal owner or child,
fixed helper bytes, inputs, credentials, server state, workloads or admission.

After attribution, add a deterministic ordering regression before implementing
the smallest test-observer correction. Missing/dead originals, unexpected child
death and cleanup must still refuse or remain explicitly negative; do not turn
cleanup into positive EXITKILL evidence. No blanket serialization, increased
deadline or automatic retry may stand in for the correction. Exercise the
unchanged default parallel group repeatedly, current and MSRV, then focused
first-exec/constructor/handoff/owner/child regressions, strict check/lint, format,
root/registry/doc/local-contract checks and expected ship refusal. Preserve failed
and successful captures, update the article and commit logical steps separately.
No push, signing, real keys/tokens, host or GitHub operations, product workload,
qualification, completion, later cells or release-status change.

Attribution: the old rooted stop returns SIGSTOP siginfo and is not a group-stop.
Concurrent negative controls observed a zombie with TracerPid0 and Seccomp2,
while the original TGID was live. Use a proven group-stop for both rooted loss
cases before their EXITKILL options diverge. Add a deterministic kernel-stop
regression (old cut must fail) and four concurrent negative controls; keep all
previous terminal/liveness/cleanup requirements and deadlines unchanged. Do not
claim a captured exact natural exit code from status alone.

The group-stop-only repeat then rejected TracerPid0/StateR with both originals
live. Treat detach and job-stop as a joint asynchronous predicate, not two atomic
observations. R/t remain pending under the original deadline, T+TracerPid0 is
required, and death/unknown state refuses. A deterministic decision regression
must fail before this polling correction. Do not add a second SIGSTOP through
pidfd; retain the pending-signal exploration as an unselected alternative.
