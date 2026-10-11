# Original signed leader mount composition: local evidence

Implementation `1a7d946ed542908ca04c1aa3d4c5572973d90c4d`, preregistration
`2f9da9e07fcce93309488b7d12ea1c80e483cac9`, baseline
`b07a655bb42237b3eb80e774b953017e39850dd3`.
Final reviewed hardening: `7cd8b43061638a66b10a3df431a642cd95f19cb3`.
See the [design](../../diagnostic-signed-mount-local-design.md),
[manifest](manifest.json) and [server checklist](../../production-readiness-and-server-launch-20261011.md).

Final scoped Linux results: 370 passes, one pre-existing system-bus ignored.
Portable Windows: 43; Linux-target composition tests execute zero there.
Root contract/evidence/governance: 135. Thirteen composition tests pass three
parallel and three serial repetitions before hardening, and three further serial
repetitions afterward; one internal adapter test also
passes. The first green/repeats have 12 tests; the later all-component context
restoration regression is explicitly retained as a strengthened suite.

Negative captures include missing API, one strict lint, two inherited source-shape
contracts, missing cargo-deny in Linux and Linux verify's Windows worktree Git
path refusal. Native Windows dependency policy passes with duplicate warnings;
it is not a warning-free gate. Native Windows full verify runs separately; its
terminal result must not be inferred from the scoped counts. Ship admission remains
expected-red. Every closed raw capture is preserved byte-for-byte.

Real owned local processes are not authenticated production starts. Synthetic
issuer/account fixtures cannot promote to production origin. Foreign namespace
mutation, all-task inspection, durable authority, production preparation,
qualification and numerical claims remain unproved/closed. No host is modified.

Run `python verify-packet.py` for read-only inventory, hashes, totals, source/test
labels, closed proof boundaries and frozen qualification bytes.

The 44-capture scope packet also retains the passing portable Rust 1.88 all-target
compile and final 28 diagnostic root guards. The final registry check passes with
per-command Linux Git directory/worktree environment after the first environment
refusal; no worktree metadata is rewritten. This scope packet is not a full
workspace milestone or a registered exact-candidate nextest receipt.
