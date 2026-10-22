# Pure authority bounded read reference baseline

Dependency-free test image, not a product library. File/Metadata/open receivers
are synthetic; only the generic std Read::take/read_to_end executes real library
code. No filesystem, real descriptor, clock, host process or authority registry
is opened. The isolated request counter is reused unchanged.

Whole copied Leaf/open/read/revalidate, Stamp/of and regular-file bodies preserve
the original sequence and ownership. The [protocol](../../docs/testing/performance/0.74/authority-read-baseline-plan-20261022.md)
fixes nine cases and 256 healthy reads per case, with separate one-call stages,
32/64 first-fault masks, metadata/bounds/ownership and synthetic receiver controls.
Standard-library Interrupted handling is not application retry or retry-to-pass.
Only main healthy rows enter the main totals; stage/fault/seeded calls do not.

Both first Windows MSVC compiler lanes pass nine tests. The host lane checks
three new source canaries plus the preceding forty-six, xtask check and strict
clippy. Read-only reconstruction and mutation tests after packet publication:

```powershell
python -B tools/authority-read-baseline-074/review_read.py --output docs/testing/performance/0.74/local-runs/authority-read-baseline-20261022/facts.json --verify
python -B tools/authority-read-baseline-074/test_review_read.py
```

The reader verifies first receipts/raw hashes, complete source pins, stage/main
conservation, exact fault/event prefixes and seventeen earlier seals/955 members.
It keeps compiler lanes separate; agreement here does not constrain other std
versions. Initial build-only E0583 was a missing controls path attribute, fixed
before either measured test selection. No measured selection was retried.

These are Rust requests in one synthetic byte corpus, not GNU/Linux File cost,
FD lifetime evidence, full-path CPU, RSS or retained memory. No storage candidate
is implemented. A transient buffer proposal remains a separate hypothesis with
its own cost/semantic gate, original deadlines and closed live/server admission.
