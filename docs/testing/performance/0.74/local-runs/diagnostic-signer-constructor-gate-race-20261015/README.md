# Constructor gate lifetime attribution

Baseline `da47b2431a3281d2cfa6f76d1020bd3ed8c43c57`; preregistration
`8d4e865c5b2f18f61bd3af536c94e5ab10709556`; cut/fault-only instrumentation
`c6df158dd3e832ff2540ef846772cf47d77c07ce`. Correction
`07b88def29d47c67574499727271f751533080fe`, tree
`da7e9255216ec976d957c3a6310d8e4afc7188cb`, changes only the private constructor
fixture, its closed contract/root guard and compatibility documentation.

The preceding packet's failed broad 78/1 log remains byte-for-byte unchanged and
is pinned against the baseline Git object. Two fresh broad groups pass79 before
correction, with and without refusal context. That does not erase the old failure.
Its failed fresh seeded loop uses fault=None/healthy=false; only before-arm
rejects the observed SIGKILL tuple. This cut identification is a closed-oracle
inference, not a captured historical cut/fault field or syscall trace.

A controlled kernel interleaving forces EOF while the original creator is still
alive. The unchanged raw guard reaches its readback/parent-check path and first
ptrace stop before EXITKILL. The monitor then kills only the original tracer;
the original child dies SIGKILL and is reaped through its retained pidfd/ECHILD.
That controlled outcome is explicitly not the before-arm exit81 proof. The same
capture retains the red barrier regression: old code emitted no retained-peer
proof even when its before-arm terminal tuple happened to be exit81.

The monitor now holds the original Hook peer, transferred before original fork,
through exact tracer termination/reap. Before-arm and no-arm controls require
the original child pidfd nonterminal before peer release. Exit81, controls83/84,
armed SIGKILL, healthy handoff, five-second per-step and 30-second outer bounds
stay exact. Setup revision2 adds exactly one `G` byte/one-FD packet, making ten
healthy packets rather than nine; the shared parser remains unchanged at
32 bytes/five FDs/128 control bytes per packet. Socket shape, flags and kernel
creator credentials are checked; this is a cooperative fixture, not authenticated
custody against arbitrary same-UID senders. All observer joins finish before
asserting results. A missing FileTypeExt import during development is retained
as a compilation failure, not a kernel result.

Final Linux Rust1.94 passes three default-parallel groups of83. Each includes
11 constructor tests, nine successful original before-arm peer barriers, exact
negative controls, one early-EOF counterfactual and four independent pre-arm
observers. MSRV Rust1.88 passes83; fixed helper artifacts still use pinned1.94.
Ordinary owner/root/GNU/isolation regressions pass54, first-exec regressions11,
Windows current/MSRV portable groups each pass7/1/1, root contracts159, strict
check/lint, formatting, registry175 and doc/local contracts pass. Ship refuses.
These are scoped correctness checks, not the whole workspace/release gate.

Sixteen saved captures include baseline, diagnostic, red, compile failure,
provisional and final results. Text is tool-returned and normalized to LF with
one final LF; their saved bytes/hashes, exit codes, summaries and proof markers
are pinned alongside source/tree, unchanged originals and frozen inputs. This is
unsigned development consistency, not execution attestation from a clean final
checkout. The verifier and15 positive/mutation tests pass on Windows and Linux:

```text
python -B verify_packet.py
python -B -m unittest test_verify_packet.py
```

The raw constructor guard, ordinary owner/root preparer, isolation installer/policy,
rooted constructor, common IPC parser, helpers, Cargo.lock and qualification
manifest are unchanged. No real credential/token/signing, host/GitHub mutation,
product workload, performance measurement, qualification or release admission.
Next is normal original-child/root/runtime composition; full-entry loss,
authenticated custody/handoff, original worker start, P2/P3 all-writer cessation
and completion/cells2–4 remain separate.
