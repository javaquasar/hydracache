# Pure proc document buffer baseline

Separate dependency-free Windows MSVC test crate; not a workspace product or a
runtime candidate. Seven tests isolate the current constructor, four synthetic
reads and drop, with separate stage, ownership/tail/error/seeded and allocator
controls. The full current core is copied exactly except for the File/read_at
adapter. Runtime trace and error receivers are synthetic, not kernel observations.
Three xtask canaries verify the whole core and unchanged runtime sources.

```powershell
./tools/proc-document-baseline-074/check.ps1 -Lane 194
./tools/proc-document-baseline-074/check.ps1 -Lane 188
./tools/proc-document-baseline-074/check.ps1 -Lane host
```

Only installed Windows toolchains are used, with offline/locked Cargo. The first
outputs refuse replacement; any repair needs another cohort. No ignored test,
WSL invocation, clock read, sender/owner, service, server or product is exercised.
The test-image allocator never links into the supervisor. The crate's empty
non-test library exposes no runtime API, dependency or binary.

The 256-instance corpus and independent one-instance stage controls must not be
pooled. Counts are Rust requests, including alloc_zeroed and drop layout, not
physical zeroing, CPU, RSS, retention or preparation headroom. This baseline
does not repeat the earlier complete Sender comparison or claim new savings.
The old per-invocation reuse optimization is already present; the remaining
allocation is eager, owned and bounded to 65537 bytes per constructor.

`review.py` verifies thirteen earlier raw seals and 770 members, seven frozen
probe source hashes, exact first receipts, compiler/platform identity, all seven
test names, corpus/seed/errors, stage conservation and unpooled totals. It also
reconstructs the preceding offline reconciliation unchanged. Eight pure mutation
controls exercise refusals without invoking a probe. For read-only replay:

```powershell
python -B tools/proc-document-baseline-074/test_review.py
python -B tools/proc-document-baseline-074/review.py --output docs/testing/performance/0.74/local-runs/proc-document-baseline-20261022/facts.json --verify
```

First publication omits `--verify`, requires the separate pre-created packet
and exclusively creates facts.json. `--packet-sha256` additionally checks that
packet's raw seal and membership, only with `--verify`. Source drift refuses;
do not relax old pins for a later candidate. On a fresh checkout, reviewer tests
read the retained first logs; before publication they read the owned target logs.

After first publication, `final-check.ps1` records formatting, eight new and
twelve prior offline controls, facts reconstruction, structural gates and exact
protected-source preservation. It neither repeats the two allocation probes
nor runs a full milestone gate. Existing output directories refuse replacement.
