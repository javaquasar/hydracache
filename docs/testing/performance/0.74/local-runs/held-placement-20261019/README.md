# Held placement: local data-only evidence

Source `11e8a588a3b05b60f5c466e3fce7ca7ecb4ab923`; baseline
`b23a503bb81c962d578143e72d0c3dbc7fd82fe0`.
Read the [plan](../../held-placement-plan-20261019.md) and
[design](../../held-placement-design.md) before interpreting the results.

This package checks a new supplied-key v3 byte reader. It binds complete nested
v2 launch claims, the exact 104-byte v2 Enroll packet and deterministic expected
slice/child placement. It does not construct a complete held unit or observe
actual systemd placement. Original owner/context/bound join, production issuer
selection and all runtime admissions remain closed. There is no server access,
installation, supervisor operation, original/system-bus send, product workload,
qualification, new fault cohort or clock adjustment in this package.

## Baseline and reproducibility

Before implementation, baseline194.log and baseline188.log run the five old
two-phase tests on retained ordinary GNU 1.94/MSRV 1.88 release libtest images.
Their exact SHA256 values are in baseline-images.log. These images were built
from `22b7a297a49665d5b7f85b0253dd982c2ab9f5c7`; b23a503b changes only docs,
evidence and attributes, not that runtime source. They are not profile-feature
images and are not product benchmarks. red-missing-module.log then records the
new tests failing to import the not-yet-implemented module. green-portable.log
is the first successful six-test portable check.

Use a separate checkout at the pinned source and the locked dependencies.
Copy only build.ps1 from this evidence revision into the same packet-relative
directory in that checkout; do not copy the existing cohort directories or
overwrite this retained packet. The local recipe refuses dirty supervisor
sources, a changed HEAD or an
existing output label. It pins compiler and ordinary release image digests and
runs strict all-target/all-feature lint, eleven launch tests (five old, six new),
five unchanged enrollment tests, original-owner/kernel/root regression and
247 Linux documentation tests. Windows has twenty. Do not run two instances for
the same compiler.

```powershell
./build.ps1 -Compiler 1.94.0 -Label gnu194-final
./build.ps1 -Compiler 1.88.0 -Label gnu188-final
```

Those relative commands refer to this packet directory; enter it only from a
fresh reproduction checkout. The recipe uses the existing local Ubuntu caches
but does not copy or install an executable outside those compiler caches.

Portable checks and structural guards run from the repository root:

```powershell
cargo clippy --locked -p hydracache-long-run-supervisor-074 --all-targets --all-features -- -D warnings
cargo test --locked -p hydracache-long-run-supervisor-074 --lib -- --test-threads=1
cargo test --locked -p hydracache-long-run-supervisor-074 --doc
cargo test --locked -p xtask --test performance_contract_074 -- --test-threads=4
cargo clippy --locked -p xtask --all-targets --all-features -- -D warnings
cargo xtask gated-test-check
cargo xtask doc-check
cargo xtask performance-contract-check --release 0.74
```

The source-only suite uses four threads after inspecting its support modules
for process-global mutations; only forbidden-operation string literals matched.
This is not timed attribution and changes no live or fault schedule. The new
adapter must reconstruct all 423 prior supervisor sources, old COMPAT sections,
registry and external creator harness exactly. Only the two additive module
hooks and new wire section are removed; unrelated drift remains visible.

The new reader tests cover 128 deterministic identities with seed 0x074e5a1c,
each of 104 packet-byte mutations, all packet kinds, zero/foreign/oversized
root claims, unknown/duplicate/noncanonical/oversized documents, bad hex,
wrong key and cross-version/domain confusion. These test cases do not constitute
live root enrollment, a renewed deadline or another creator SIGKILL proof.

The initial gnu194/gnu188 runs actually pass all 247 Linux doc tests, but their
wrapper wrongly expects the Windows count of twenty and exits with Wrong test
selection before producing a receipt. build-initial.ps1 preserves that exact
recipe, and initial-wrapper-outcomes.json records this tooling failure. Only
the wrapper's Linux selection count is corrected; all runtime/test source and
original deadlines stay unchanged. Fresh -final directories retain the complete
corrected recipe results, without overwriting either initial run.

## Integrity and interpretation

Final checks pass: the complete portable Windows library has 60 unit and
20 doc tests; each GNU 1.94/MSRV 1.88 ordinary release image passes eleven launch,
five enrollment, two owned-manager, 22 origin, four closing and nine root tests,
plus all 247 Linux doc tests. Two origin and three root opt-in tests remain
ignored as before; no new fault cohort is inferred from these selections.
All 309 source contracts, the three new structural/mutation guards, strict
supervisor/xtask lint, formatting, all 184 registry gates, docs and the local
non-promotable performance contract pass. The prior boot packet verifier also
passes its sixteen mutations without changing that packet or qualification.

The packet-local attributes preserve raw bytes. verify.ps1 inventories every
file except inventory.json, checks source/image/recipe and successful test
receipts, pins the unchanged prior boot packet inventory and qualification
manifest, and refuses sixteen metadata/inventory mutations. It refuses resealing.

```powershell
./verify.ps1 -SelfTest
```

Do not infer preparation-wall, allocation, RSS or product-speed improvement from
these results. The prior allocation and six-case MSRV creator-fault evidence
remain separate immutable packets. Next is the same-owner live join and a
complete authenticated held-unit projection before delivery, not sending the
old direct-worker message using a newly signed placement claim.
