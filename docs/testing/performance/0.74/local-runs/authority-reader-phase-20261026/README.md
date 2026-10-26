# Authority reader phase evidence

This source-only step closes five remaining failures in the selected seven
authority/current-tree targets. The first baseline has 17 passes and five
failures; the first candidate has 25 passes and two failures at the newly reached
terminal main-test source. The final candidate passes 22 old and six new cases.
Five unchanged successor controls bring coverage to 33 distinct cases on each
of Rust 1.94 and MSRV 1.88, with no ignored/filtered cases.

The current-compiler coverage comprises the 28-case candidate and a separate
five-case successor cohort on the same compiled sources. MSRV runs all 33 in one
invocation. Do not add these numbers to the interrupted full package's coverage.
The full package has not been repeated or declared green.

The immutable index binds raw check logs, before/after source and HEAD receipts,
fixed original readers, declared deltas, final source, both fixed terminal-main
producers and local coordinators. First-variant input hashes are retained, not a
claim to reconstruct every transient body. The original package's complete
authority-context failure section is a derived excerpt with original stdout
hash/offset/length, not a new run. It includes expected mutation panics followed
by the actual registry refusal.

The index binds 74 members in 63 distinct raw objects, with SHA256
`8022ce3470effdd0f4c1df62747cf076f2c42c5420d6b3146b08c70534a03417`.

`verify.py` checks raw objects, exact case/outcome accounting, unchanged source
within every invocation, final current/MSRV source bindings, independent old
reader hashes, and all five inverse main-source edits. It does not execute
retained Rust/Python code, Git, Cargo, fixtures or workloads. Run it and its
mutation controls offline:

```text
python docs/testing/performance/0.74/local-runs/authority-reader-phase-20261026/verify.py
python docs/testing/performance/0.74/local-runs/authority-reader-phase-20261026/test_verify.py -v
```

No production credentials, private enrollment keys or binaries are retained.
Step wrappers keep the 1,200-second
bound/four-GiB disk floor; the full-package 780-second guard is unchanged. No
runtime/live-check/deadline, dependency, clock, service, server or qualification
change occurs. The [result report](../../authority-reader-phase-result-20261026.md)
defines the exact scope and remaining package boundary.
