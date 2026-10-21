# Original owner materialization refuses full revalidation at the original bound

The first whole original-owner projection/materialization gate **did not pass**.
On both GNU 1.94 and MSRV 1.88, production projection and materialization returned,
including the twenty-FD retained inventory. The subsequent production material
revalidation refused with the original `Causal(Time)` error. Neither compiler
was retried. No runtime check or original 300/60/10-second window changed.

The [preregistered method](held-material-live-plan-20261021.md) is frozen at
`1d6d179bd7150ef564560821a897794e20b24b36`; runtime remains the materialization
candidate `d1bf3d00edb94cc22c3163ac26e8f6d955d2a2b2`. The only additions to that
runtime are a test-only module hook and an exact opt-in branch after the old
consuming join. Source canaries reconstruct both previous complete files at
`215626cb` and all historical validators. The existing manual gate is reused;
default fixture cases and all old assertions remain unchanged.

## First retained outcomes

| Compiler | Child wall seconds | First refused original age ns | Combined new phase wall ns | Causal step calls and completed |
| --- | ---: | ---: | ---: | --- |
| GNU 1.94 | 12.786527020 | 10042390247 | 4418926816 | 56 and 55 |
| MSRV 1.88 | 12.042026637 | 10010835739 | 4653421864 | 59 and 58 |

Each child is a distinct first fixture invocation. The original admission origin
is `463631383166` ns for GNU and `476668137622` ns for MSRV; first refused samples
are respectively `473673773413` and `486678973361` ns. Subtraction reconstructs
the retained ages, both beyond the unchanged 10000000000 ns causal window.
Issued and observed boot IDs match the session boot
`6a709ad1-c736-45a2-bd6a-351e07f6b8f3`. The error comes from the original causal
predicate, not the wrapper timeout, new inventory rejection or changed deadline.

All ten preceding preparation phases returned. In the eleventh phase, the
markers stop after `projection-enter`, `projection-return` and
`material-return descriptors=20 original_bounds=true`. Both refusals occur at
the adapter's single `material.revalidate()` call. No `revalidate-return`,
`drop-return` or old whole-fixture success footer appears. Thus original-owner
construction returned, but the complete construction/revalidation/cleanup gate
remains closed; unit mechanics success cannot repair this result.

The combined new phase includes projection, materialization, attempted
revalidation, observations and refusal unwinding. Its wall time is not the
exclusive cost of allocation, memfd sealing or descriptor census. There are no
separate stage timing samples. The conserved raw kernel graph contains 220128
Boot and 440256 Namespace observations in GNU's combined phase, and 232536 and
465072 respectively in MSRV's. These are nested calls in different refused
prefixes, not independent operations, a compiler comparison or a measured
optimization opportunity. Further attribution must distinguish repeated full
frames from effects without removing fresh checks or resetting origins.

## Context and cleanup evidence

Metadata, both profiles and postflight share one WSL parent, PID 412/start ticks
45919. Every before/after snapshot is identical: exact clean committed source,
both default-feature image hashes, original nine-file synthetic bundle,
qualification, boot, parent identity and configuration. Wrapper failure is null;
both Rust exits are 101 and the launcher correctly exits 1 with
`schedule_completed=false`. No matching fixture residue is observed afterwards.

The refusal unwinds the retained owner and the existing capture confines and
terminates only its own child process group. However, panic bypasses the old
post-drop cgroup/custodian/EOF/fence assertions. Empty wrapper residue does not
prove those assertions, safe unlock or cessation of arbitrary writers. They are
not relabeled as passed. There is no original message send, release, product,
completion, server action, clock/service/idle-setting change or installation.

## Verification and retained evidence

Before live entry, GNU and MSRV each pass 82 selected tests: twelve material,
ten projection, ten consuming, twelve reply and thirty-eight held-build tests.
Fourteen source canaries pass. Both Linux compilers and Windows pass affected
all-target checks and strict all-feature clippy; formatting, doc-check, all 186
registered gates and the local non-promotable performance contract pass.
The new nine offline scheduler/reader controls pass on Windows and Linux.
The unchanged runner passes all 29 Linux controls; Windows passes 22 with seven
Linux-only controls skipped. These are not extra live attempts.

The initial check-script invocation via Windows PowerShell 5 refused a native
compiler stderr progress line before recording a check result or starting any
original fixture. Its empty logs and refusal receipts remain in `checks/`.
The frozen method requires PowerShell 7.2; complete check logs are separately
retained under `checks/pwsh/`. This tooling correction did not replace a live
outcome. The single actual profile session starts only after source freeze.

The [packet](local-runs/held-material-live-20261021/README.md) retains raw first
logs, complete contexts, build/image receipts, launch receipts, preregistered
method copies, check logs and independently reconstructed summary. Review never
invokes a Rust fixture. The qualification manifest remains byte-identical at
SHA256 `11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

The packet contains 65 members plus its inventory, whose raw SHA256 is
`0c385fbfe209fad1903aa2c0e620817c458f413eec8bd36fa32be5c01fbb4109`.

Next attribute the retained combined phase offline and design a separately
reviewed cost candidate while preserving every fresh predicate and original
origin. Do not repeat this packet to obtain a pass. Durable single delivery and
uncertainty, actual process/barrier binding, active collection, all-writer
cessation, completion/cells 2–4, full joined SIGKILL/lost-ACK/reconnect and
production enrollment/server rehearsal remain independent closed gates.
