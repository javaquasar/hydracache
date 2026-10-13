# Local authority quarantine process loss evidence

This packet tests actual SIGKILL during the existing owned Starting quarantine.
It verifies retained attempt and authority objects, common-fence release,
exact owned helper termination and fresh read-only observation. It does not
authorize execution, recover a runtime acknowledgement or establish cleanup.

## Source and reproduction

Baseline is `b6c76476be7b99117a267dfcd570e4dffa8c7e00`; preregistration is
`c2638542d06287ba40ca0a36d6659579cfe60f21`. The manifest pins the final
implementation and five source/registry files. The production engine differs
only by its cfg(test) observer declaration and call; removing those additions
must reproduce the exact baseline source. Existing quarantine, original
authority bindings, write order and schemas are unchanged.

The implementation is `86a52311a4094d657a0ac59ede3310f1731b581d`.

Run `C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-quarantine-crash-20261013/verify_packet.py`
to inspect this packet read-only. The local Linux manual command is
`cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --locked --lib
diagnostic_process_crash_quarantine_all_stages_and_post_return --
--ignored --nocapture --test-threads=1`. Its gate remains manual with seed
0x074d and the unchanged 600-second harness budget. Ordinary fast tests skip it.
Do not relink the active libtest executable while its owned child proofs run.

## Boundary observations

Each repeat covers BeforeRefusal, RecordCreated, RecordSynced, MarkerCreated,
DurableCommit and AfterReturn SIGKILL, plus an uninterrupted AfterReturn control.
At BeforeRefusal the original authority is still active, while retained
reservation context already prohibits a new attempt. The first three append
boundaries retain invalid journal tails. DurableCommit and AfterReturn retain
a complete sealed revision-two refusal, not the dead owner's acknowledgement.

Two fresh processes per case inspect both the original reservation and authority
journal without mutation. Their fixture policy pin comes from the test parent's
original fixture record, with the known fixture issuer and local host/namespace
context; this is not independent production enrollment. No partial or complete
tail is repaired, appended, retried, deleted or promoted to a start capability.
Retained bytes and metadata must remain exact after termination and each read.

The hook's fast tests cover fault preservation, actual four-stage append order,
scope restoration on return/panic, nested-scope rejection and independent
threads. Its module and observer call are cfg(test) only. A separate normal
library build and read-only nm receipt confirm observer symbols are absent for
that exact build/config; no production activation is performed.

## Retained development results

Baseline passes the eight existing quarantine cases. The missing-hook red test
fails with E0433. The first manual invocation fails compilation with E0716 due
to a borrowed temporary fresh-inspection label; it executes no crash matrix.
The corrected harness completes an initial six-kill/control matrix.

Root contracts then fail because an early cfg(test) declaration truncates a
prefix-based source guard before its existing lock markers. Moving the isolated
declaration beside the old test blocks restores the same guard without changing
locks, budgets, storage operations or assertions. The initial positive matrix
remains development evidence; three later repeats use the final source.
No failed invocation is relabeled green. Intentional caught panics in fast hook
tests are scope-restoration assertions, not a failed process-loss run.

## Closed admission

Scoped Linux regression passes 673 tests with nine manual/host cases ignored.
The eight ignored process proofs are exercised separately; the real-host bus
case remains unrun. Windows passes 204, root contracts 166 and Linux compile-fail
doctests 52. Five new hook tests and all eight existing quarantine cases pass.
Current/MSRV checks, warnings-denied lint/Rustdoc, format and the local
performance-contract gate pass. Documentation/book and artifact checks are
included in the final packet.

Three final-source matrices each pass six SIGKILL cases, one uninterrupted
control and fourteen fresh inspections: eighteen signal cases, three controls
and forty-two inspections in total. The full eight-proof regression additionally
passes 72 SIGKILL cases, seven controls and 158 fresh inspections. These are
separate execution groups, not merged or transferred numerical samples.

This is actual local process loss, not physical power loss, host reboot or
positive installed-root enrollment. Recovered journal views are observation
data only; no missing PID/inode proves writer cessation or cgroup emptiness.
P1 still requires reboot semantics, authentic completion and later fixed cells.
P2 through P5, numerical pilot and product qualification remain closed.

Qualification dry-run and require-ship remain expected exit-one refusals, with
no requested qualification artifact. The raw manifest digest and eight frozen
inputs are unchanged. No product performance measurement or claim is made.
