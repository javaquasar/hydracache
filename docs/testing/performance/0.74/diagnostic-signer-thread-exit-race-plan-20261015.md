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
