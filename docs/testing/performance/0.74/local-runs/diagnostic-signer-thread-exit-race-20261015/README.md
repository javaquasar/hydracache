# Rooted tracer thread loss without a scheduling race

Implementation `9b9ee382939923bb6a03ca5564d10a8065829b21`, tree
`fce5bb6e1ff812d66e749ccbdbd5b7bf07ccff77`; preregistration
`7072f5e04bb9a4b789caf462e79ff0995c7490ab`, baseline `7eca2350`.
Unsigned development captures establish local consistency, not clean-checkout
execution provenance, authenticated signer custody or release admission.

The old negative control could resume its detached child from a SIGSTOP
delivery-stop. A concurrent failure captured original child pidfd terminal,
State Z, TracerPid0 and Seccomp2 while the original TGID remained live. The
unchanged helper's nonblocking receive without an offered dummy is consistent
with natural termination; the exact exit code was not captured. The fresh14-test
baseline and first instrumentation run passed, so success alone did not close
this race. The previous packet's two13/1 failures remain unchanged and are hashed
as historical negative evidence.

Matched rooted positive/negative controls now inject the pending SIGSTOP and
require a kernel group-stop, distinguished by exact GETSIGINFO failure/EINVAL.
They use identical preparation before EXITKILL options diverge. The first
correction passed twice, then exposed TracerPid0/StateR with both pidfds live.
The final observer waits for detach AND StateT within the existing5s deadline.
R/t only keep the wait pending; they never qualify cleanup. Death, unknown or
malformed state refuses. A second pidfd SIGSTOP was explored but not implemented.
No helper bytes, normal owner, namespace constructor, keeper, raw child setup,
terminal tuple, deadline or production API changed. Positive loss observers do
not signal the child; negative cleanup remains explicitly non-positive.

Final checks:

- Five default-parallel runs each pass17 creator/thread-exit tests, including
  six existing positive cuts and healthy controls, four independent negative
  observers,25 result/errno combinations and six joint-transition models.
- Five separate runs of the four-negative-observer test pass without serializing
  observers. Current Rust1.94 check/strict Clippy and MSRV1.88 check/17 tests pass.
  The unchanged fixed GNU helper is still compiled with pinned Rust1.94 in both.
- Unchanged first-exec/constructor/rooted-exec/namespace/handoff groups pass
  11/7/11/13/13. Ordinary owner/child/CLI/policy groups pass12/5/7/1/5 in Cargo's
  emitted order. Windows current and MSRV portable policy/CLI pass7/1 each.
- Root contracts158, registry174, formatting, strict Windows lint, docs and
  non-promotable contract pass. Require-ship still refuses with exit1. No full
  workspace/release gate, cache-performance workload or server qualification ran.

The deterministic kernel-stop regression was red before the first correction;
the transition model was red before joint polling. Both failed corrections and
the pending-signal exploration remain captured. Prior packets are historical,
not rewritten to claim this fix. Local kernel observations are not guarantees
for all Linux kernels. Normal root/runtime selection, full normal-entry loss,
credential handoff, authenticated worker start and all-writer fencing remain open.

Reproduce on nonroot Linux x86_64 with Rust1.94 or Rust1.88:

```text
cargo test -p hydracache-long-run-supervisor-074 --lib diagnostic_signer_root_fixture::rooted_exec::handoff::loss::creator_loss:: --locked --offline -- --nocapture
```

Do not add `--test-threads=1`, retry failed runs or increase deadlines. Run
`python -B verify_packet.py` and `python -B -m unittest test_verify_packet.py`
from this directory. The verifier checks raw hashes, source/tree/frozen inputs,
unchanged oracles, historical negatives, exact test counts and closed claims.
