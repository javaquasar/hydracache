# W14 coverage, canary and admission evidence

W14 is implemented as release machinery, not as a claim that 0.72 is already shippable. The
candidate command remains intentionally red until all external exact-candidate inputs exist.

## Implemented gates

- `management-center-check` parses the claim, source-map, coverage, canary and failure-taxonomy
  registries. It rejects duplicate or missing IDs, unsafe/missing symbols and tests, unowned routes,
  unknown canaries, incomplete status, invalid exact-candidate receipts, partial taxonomy and
  coverage floor/module drift.
- Claim receipts are strict JSON envelopes, never committed prose. They bind the claim ID, work
  item, implementation/test/canary mapping and registry digest to one clean 40-hex source commit.
  Their proof-input set must exactly equal the work item's fast gates, gated gates and dynamic
  canary. Every referenced receipt is re-parsed and revalidated for command/registry/input/artifact
  digests and expected-red outcome; existence or a self-reported `pass` is insufficient.
- After all underlying receipts exist for a frozen clean candidate, generate the derived claim
  receipts with `cargo xtask management-center-check --release 0.72 --write-receipts
  --receipts-dir target/release-evidence/receipts`. Any dirty, stale, missing, duplicated,
  path-traversing or hash-mismatched input makes generation/admission fail closed.
- Every NATS-derived failure-taxonomy row names its owning W-item and may reference only a validated
  claim receipt owned by that same item. A prose note, receipt from another item, or canary whose
  ID has another owner is rejected before ship admission.
- Pre-feature and published-0.71 baseline files use a deny-unknown-fields JSON schema. Admission
  resolves the source ref to its exact commit, binds it to the clean candidate SHA, requires outcome
  `pass`, and requires each declared measurement exactly once with a finite non-negative value,
  reviewed unit, non-zero sample count, and command/output SHA-256 provenance. File existence alone
  is never baseline evidence.
- `canary-registry-0.72.json` registers W0-W14. `canary-sweep` runs the unchanged selector first and
  then one reversible defect, requiring the exact `HC-CANARY-RED` marker and writing a clean-SHA,
  command-digest and registry-digest receipt.
- `release-evidence/0.72.toml` maps every work item to sources, executable Rust tests, artifacts,
  `fast.workspace-nextest`, and the applicable daemon/resource/coverage gates.
- Four dedicated cargo-fuzz entry points cover the management envelope, durable recovery,
  placement trace and opaque cursor decoders behind a 16 KiB input ceiling. Each is registered as
  its own ship-mandatory `tool.cargo-fuzz.management-*-072` gate and runs through `evidence-run` in
  scheduled/tag CI. The fast corpus gate replays committed valid and hostile seeds and asserts the
  oversize short-circuit; scheduled candidate runs retain four distinct time-bounded libFuzzer
  receipts, so one green decoder cannot stand in for another.
- `release-evidence --release 0.72 --receipts-dir target/release-evidence/receipts --require-ship`
  invokes strict management admission before it can
  aggregate ordinary gate receipts. The `MC72-W14-PAPER-GREEN` test proves this path cannot bypass
  missing semantic evidence.
- `cargo xtask verify` and the Linux CI Rust job run the structural management check; CI also runs
  all 15 release-scoped expected-red work-item proofs and emits the non-promoting evidence report.
- The complete dependency policy (`advisories`, `bans`, `licenses`, `sources`) is enforced rather
  than the former bans-only subset.

## Executed development proof

The following checks are green on the implementation branch:

```text
cargo test -p xtask --test management_center_072 --locked       14 passed
cargo test -p hydracache-fuzz --test fuzz_corpus_regression --locked 4 passed
cargo xtask management-center-check --release 0.72              OK
cargo xtask canary-check --release 0.72                          OK
cargo xtask canary-sweep --release 0.72 --tier fast              15 ExpectedRed
cargo test -p xtask --test release_evidence --locked             10 passed
cargo test -p xtask --test doc_check --locked                    16 passed
cargo deny check                                                 all four checks OK
```

The implementation branch was also exercised as one full Windows development contour, with build
artifacts isolated from retained release receipts:

```text
cargo test --workspace --exclude xtask --locked -j 1             all test and doc-test binaries passed; declared external/chaos tiers remained explicit ignored gates
cargo test -p xtask --lib --tests --locked -j 1                  84 unit tests and every integration suite passed
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps       passed for the complete workspace
npm --prefix console ci                                          0 vulnerabilities
npm --prefix console run build                                   deterministic seven-asset embedded bundle passed
npm --prefix console test                                        16 unit + 4 package/supply-chain + 46 Playwright passed
```

This development contour proves source, schema, integration, process, fuzz-corpus, documentation
and browser compatibility on Windows. It does not replace the separately registered Linux
coverage/resource gates, the shipped-predecessor mixed-binary gate or either wall-clock soak tier.

The canary receipts are generated under `target/release-evidence/canaries/`; writing them into the
runtime branch would make that checkout dirty and invalidate admission. The accepted clean-SHA
copies are therefore retained only on the separate evidence archive branch together with their
registry and command digests.

## Completed promotion evidence

Exact candidate `24927c28c279c6c34ad90111ee6470b4065e0815` supplied every previously open input:

1. full campaign `35537203094` completed 50 jobs without failure, including the mandatory real
   0.71/0.72 mixed-binary upgrade, leadership-change, peer-restart and rollback proof;
2. the same campaign passed the non-shortenable six-hour candidate soak on the admitted host;
3. Linux FD/RSS, full workspace LLVM coverage and all four decoder fuzz receipts were validated for
   that SHA;
4. ship campaign `35556487047` passed the separate 24-hour confirmation with the original resource,
   traffic, TTL and hourly-fault contract;
5. baseline campaign `35723493297` supplied the clean pre-feature and published-0.71 measurements;
6. strict `management-center-check --require-evidence` and `release-evidence --receipts-dir
   target/release-evidence/receipts --require-ship` accepted all 15 work items as `ship-ready`.

Quiet skip, a development branch standing in for v0.71, a dirty receipt, a stale commit, or a retry
that overwrites a failed attempt remains non-evidence. The accepted receipts and original key
artifacts are retained on `evidence/0.72/management-center-ship` at
`351766af9241399306a385298f0cc252aca6036c`.

## Documentation and campaign-result rule

The public documentation summarizes the release contract in
`docs-site/src/reference/release-0.72-verification.md`; this engineering record, the registries and
the executable verifiers remain authoritative. A documentation page can explain a gate but cannot
satisfy it.

Every dedicated-host attempt must preserve the following chain:

1. workflow run ID and attempt number;
2. clean 40-hex source commit and exact candidate binary/UI/schema digests;
3. gate ID, normalized command digest, fixed seed and admitted host fingerprint;
4. start/end timestamps, protocol sample counts, fault/recovery observations and resource series;
5. final outcome plus immutable links or digests for logs and uploaded artifacts;
6. predecessor receipt when the attempt is a retry.

Failures found during the 2026-09-19 campaign and their corrections are retained in the W12
hardening record. They are not converted to passes. In particular, the six-hour run on
`ad9fd2b813b55fe44e70c2cea692495496e50ff2` can support only that commit. Any subsequent source or
documentation commit creates a new candidate SHA and therefore requires fresh exact-SHA receipts
before promotion.
