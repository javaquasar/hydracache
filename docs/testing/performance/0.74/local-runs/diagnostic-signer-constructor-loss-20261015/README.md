# Original parent loss before EXITKILL

Baseline: `e4eaea4cd4af5e09082cf8cb8b759ff40ae09d7c`.
Source: `815fc5d7527153387b3b4c095c035cda03df523e`.
Preregistration: `6a02eeef9e09b319f2711953b80a9afb694be0b1`.
Healthy-progress amendment: `32237eb368e230f72fca636747cbc06984bf85ed`.
The manifest binds the final source commit separately from this evidence commit.

## Confirmed scope

Local nonroot WSL Ubuntu UID/GID 1000, Linux x86_64 kernel 6.6.87.2, current
Rust 1.94 and MSRV 1.88. The selected GNU helper remains Rust1.94/O3/panic-abort
with unchanged ELF bytes. Windows validates portable tests and fixture exclusion.
There is no dedicated-server, workflow, real key/token or product workload run.

The existing original builder now captures its parent TGID before fork. Its raw
child arms and reads back PDEATHSIG(SIGKILL), then checks the original parent
before descriptor/root/credential changes. The guard overlaps the original
owner's EXITKILL installation and clears only after the first resume. The old
post-options tracer-loss controls still use their unchanged separate oracle.

Three seven-case repeats cover five constructor cuts. Before arming, actual
whole-parent SIGKILL must produce original-child CLD_EXITED/81 through the
nonretroactive-signal parent check. The four armed cuts require original-child
CLD_KILLED/SIGKILL. The independent observer signals only the original tracer
pidfd, consumes the original child's exact pidfd record and requires ECHILD.
Original runtime directories are released only after that terminal reap. No
positive or negative loss case signals the child or sends a dummy credential.

Each repeat includes eight seeded fresh cases, four independent observers, five
healthy original ACK/close/zero-exit controls, two separate negative controls
and finite terminal/frame mutations. Across three repeats there are 51 positive
terminal records (12 parent-mismatch exits and 39 SIGKILL records), six negative
control exits (83/84) and 15 healthy completions. No negative control can satisfy
the positive oracle. Emergency cleanup never counts as proof; a failed attempt
can leave owned temporary forensic files rather than a positive cleanup claim.

The final exact suite vector is
`7,7,7,11,13,13,11,7,10,9,11,8,18,12`: three new repeats and all 123 previous
regressions, with no zero-test suite. MSRV passes seven new tests and current
package check/strict lint pass. Windows portable tests pass nine. Source
contracts pass 146, the unchanged gate registry passes 169, scoped rustfmt and
documentation pass, and ordinary library symbols exclude the fixture. Local
performance-contract admission remains non-promotable; require-ship remains red.

## Failed and incomplete development observations

The initial new tests fail E0425 before implementation. The first implementation
fails on wrong method names and an unmarked unsafe borrow (E0599/E0133). The
next five-case and seven-case current runs pass, before healthy progress is added.
They are historical development results, not the final source gate.

The first MSRV pressure capture passes six and fails the healthy control after
root descriptor checks. Its aggregate five-second wait covered the entire
remaining bootstrap. The separately committed amendment adds seven exact,
credential-checked, descriptor-free success hints for builder/root/exec/install/
READY/delivered/complete. No five-second gap, 30-second outer bound, positive
terminal predicate or final cleanup check is increased or weakened. The final
MSRV passes with those hints. Aggregate-wait structure is confirmed; pressure
attribution for the earlier failure remains a hypothesis, not a kernel defect.

Orchestration mistakes are also retained. The baseline cargo suite passes 11,
but Windows-to-bash expansion breaks its PIPESTATUS wrapper and prevents the
chained check; the check is rerun independently and passes. A later shell loses
its variable filter and is stopped before an unintended unfiltered suite starts.
Another explicit but incorrect filter selects zero rooted tests, an overly broad
62-test group, eight rather than 18 bundle tests and zero producer tests. The
Windows lib filter likewise selects zero rather than the intended integration
test. The verifier rejects zero/mismatched suite counts in final evidence; the
correct full filters and Windows integration target are rerun. The mistaken
`gate-coverage-check` command refuses; actual `gated-test-check` passes. These are
verification failures/selection gaps, not product failures or admitted passes.

## Reproducibility and admission

The manifest binds 12 source files, 65 unchanged files, 32 raw development/final
captures plus two packet-check captures, the unchanged qualification digest and
16 closed admission claims.
Only parser visibility and a two-line child-module declaration change in the
previous loss observer; the verifier reconstructs both old bodies exactly.
The helper, policy, production ancillary receiver, original root, trust inputs,
dependencies and qualification bytes are unchanged. Packet-local attributes
preserve raw bytes, including original whitespace in failed captures.

Run `verify_packet.py`, `test_verify_packet.py`, and then
`verify_packet.py --committed <full evidence SHA>`. The ten mutation/control
tests pass on Windows/Linux and cover raw/self-consistent evidence drift, empty
final suites, source and closed-claim forgery, missing captures, old-parser
changes, attributes and exact
committed bytes. Auxiliary Windows/Linux packet-check logs are retained after
those checks; they do not replace source tests or increase admission.

This proves whole-parent-process loss, not pre-arm creating-thread-only death
while other parent threads live, TID reuse or arbitrary multithreaded signers.
The next launch-lifetime step must explicitly select and prove that thread
model. Independent runtime/job/issuer custody, actual credentials/token producer,
authenticated receiver/replay/fence, worker original-start and P2/P3 all-writer
cessation, completion/cells 2–4, server rehearsal and qualification remain closed.
No cache throughput, allocation, latency or production-readiness claim follows.
