# Constructor gate EOF and original process lifetime

Baseline `da47b243`. The preceding normal-root packet preserves a broad old-root
78/1 failure: constructor expected an exact refusal tuple but observed SIGKILL.
The message omitted cut/fault identity; a later focused seven-test pass is not
attribution. Preserve the prior packet and its failed log without rewriting them.

First run the same broad default-parallel group, then add bounded cut/fault/healthy
context to terminal refusal only. Do not alter the guard, helper, parser, root,
normal owner/preparer, exact terminal tuples, deadlines, seeds or qualification.
Reproduce with attribution before selecting a correction.

Hypothesis: closure of the tracer's gate peer is not a terminal-lifetime barrier.
During task exit the child may see EOF, arm PDEATHSIG, then lose its original
creator and receive SIGKILL. This can be safe guard behavior but is not the exact
before-arm exit81 observation the fixture claims. Do not widen the oracle to
accept SIGKILL at that cut or treat cleanup as proof.

If attribution confirms this boundary, retain the original gate peer in the
independent monitor until the exact original tracer has terminated and been
reaped. Keep the existing five-FD report and bounded shared IPC parser unchanged:
one additional credential-checked one-byte/one-FD setup packet may transfer the
original peer before original fork. Healthy controls keep their original grant.
At unarmed cuts require the original child pidfd still nonterminal after original
tracer reap and before releasing that peer; only then may EOF release the child.
Keep exit81, controls83/84, four armed SIGKILL tuples and ECHILD exact. Reject
foreign/wrong-type/extra gate inputs and unexpected terminal children.

Before correction add deterministic kernel EOF-versus-live-owner control and a
regression requiring the retained peer barrier (red before implementation).
Check setup ordering, lifetime and cleanup with healthy controls and four
independent observers; join all observer results before asserting. Do not serialize
the broad group, retry individual failures or extend deadlines. Retain failures
and repeated broad/current/MSRV results. Run normal owner/preparer/GNU/isolation
regressions, check/strict lint, formatting, root/registry/doc/local-contract gates
and expected ship refusal. Capture unsigned source/tree consistency and explicitly
qualify any remaining unassigned failure.

No production code, normal CLI/policy, credentials, signing, server/GitHub action,
workload, qualification, original-start, all-writer, completion/later-cell or ship
admission change. Root/normal child composition follows this attribution; it is
not part of this isolated fixture correction.

Attribution: fresh baseline and instrumented broad groups both passed79, so the
old failure was not reobserved in those runs. The retained fresh seeded failure
uses fault=None/healthy=false; only cut0 rejects SIGKILL in its closed oracle.
A deterministic counterfactual now forces gate EOF while the original creator
is alive. The same raw guard reaches its readback/parent-check path and first
ptrace stop before EXITKILL, then original creator SIGKILL produces child SIGKILL.
This confirms the EOF-versus-lifetime ordering defect, not an exact syscall trace
of the old failed run. The before-arm retained-peer regression was red before
the correction. No production guard change or broadened exit oracle is selected.
