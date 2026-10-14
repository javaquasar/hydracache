# Local signer executable object evidence

The new borrowed guard compares a retained bundle binary with the actual kernel
procfs executable object under an existing ProcessRead. Observed different-inode
exec and process/file-input loss refuse wrapper and bundle. An actual same-inode
re-exec still matches, so this packet does not authenticate original invocation,
loaded instructions, runtime/bootstrap, selected issuer/job or secret authority.
Exact source/baseline/preregistration, capture and unchanged/frozen hashes are
retained in [manifest.json](manifest.json).

## Baseline and observed outcomes

Baseline passed ten original bundle and ten legacy process-IO cases before the
new API refused at missing types/method (E0432/E0599, outer WSL exit 1). The
first eight-case run passed with one unused test import warning. That warning
was corrected, and descriptor closure assertions were completed in a cfg(test)
seam before strict lint. No suppression or predicate/handshake-budget change.

Three final eight-case Linux repeats pass. Every repeat observes a controlled
different executable under unchanged generation and rejects it; every repeat
also retains the accepted same-inode re-exec counterexample. Seed `0x07403b24`
adds 16 fresh controlled observations and four independent originals. Only
owned copied-shell helpers are spawned, with a cleared environment and a fixed
stdin protocol; they exec a fixed cat or the same owned copy and exit on EOF.
No signals, product processes, service/cgroup mutations, key or token reads.

The executable-only internal case passes invalid descriptor checks. Bundle
regression now selects 18 cases (ten existing plus eight new), while legacy
process-IO keeps ten. Linux also passes nine portable signer, 29 artifact,
eight reviewed-trust, eight API and 12 producer cases, plus 98 doctests. Windows
passes nine portable signer, 15 artifact and eight reviewed-trust cases.
Affected package check/strict Clippy on Rust 1.94 and MSRV 1.88 pass on both
platforms. Root tests pass six registry, 136 performance-contract and 13
release-evidence cases with xtask check/strict lint, formatting, doc-check and
the unchanged 169-gate registry.

Local performance-contract checking remains non-promotable; require-ship retains
expected exit 1. This is not a benchmark, full release gate, original-start
rehearsal or qualification. Existing signed schemas/pins, public process scope,
IO predicates, producer/CLI/workflows, locks, matrix/thresholds and raw
qualification remain unchanged. IO changes are only parent-module helper
visibility for the new observer; the predicates themselves are exact.

## Read only packet verification

Run `python verify_packet.py` and `python -m unittest test_verify_packet.py` in
this directory. They check current LF source against its retained commit,
unchanged baseline files and original observer locks, raw capture/qualification
digests and explicit false admission fields. Mutations use owned packet copies
only. Later source changes require checking this evidence at its retained source,
not rewriting old packets.
Six copied-packet tests pass on Windows and Linux. Their first retained captures
cover 19 build/gate logs; adding those two test captures brings the final packet
to 21 logs, with 14 tested-source and 46 unchanged-file bindings.

## Remaining pre exposure boundary

Matching a kernel executable object is not authentication of loaded pages,
interpreter/loader/libraries/JIT or the original exec invocation. The observed
same-inode counterexample is a release boundary, not something to waive. Existing
ProcessRead's production scope is a diagnostic worker, not an enrolled controller
job; tests use its existing private helper seam. Actual issuer/pins/ancestry/job
custody and runtime/bootstrap context remain unselected or opaque. No start,
signing/key grant, replay/fence, all-writer cessation, later cells or server
qualification follows from this guard. The next pre-exposure proof needs an
independent original invocation/runtime chain, not a broader interpretation of
these successful object observations.
