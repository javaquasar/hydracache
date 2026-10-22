# Pure sample boot ownership baseline

Separate dependency-free Windows MSVC test crate. The complete sample body,
point type/getters and boot decoder are source-matched; Origin, error, trace and
clock receivers are synthetic. Symbolic clock selectors do not link to libc.
The unchanged test-image allocator is imported from the proc document baseline;
it never links into the supervisor. No runtime representation or API changes.

```powershell
./tools/sample-boot-baseline-074/check.ps1 -Lane 194
./tools/sample-boot-baseline-074/check.ps1 -Lane 188
./tools/sample-boot-baseline-074/check.ps1 -Lane host
```

The first outputs refuse replacement. Each installed Windows compiler runs seven
pure tests: five ownership/frame/decoder controls plus the two inherited allocator
controls, all offline/locked. Nine cases with 256 sample calls each include
generic non-admitted strings; only fixed/fixed-spare represent the valid boot
shape. Independent one-case stages and the fixed subset are not added to the
main corpus. Sixteen fault masks test first-error prefixes, not actual kernel
faults. Seed `0x074b0a19` covers 256 owned points and independent point clones.

Counts are Rust requests and drop layouts, not physical copies, CPU, RSS,
retention or preparation headroom. There are no ignored tests, WSL/live profiles,
real clocks, installation, service, server or product operations.

```powershell
python -B tools/sample-boot-baseline-074/test_review_boot.py
python -B tools/sample-boot-baseline-074/review_boot.py --output docs/testing/performance/0.74/local-runs/sample-boot-baseline-20261022/facts.json --verify
```

The reviewer verifies fourteen preceding raw seals/813 members, the preceding
reviewer's identity, probe source pins, exact first receipts, compiler/platform,
seven test names, every case/stage/seed/fault/decoder row and unchanged original
refusals/windows. It never invokes a probe. First publication omits `--verify`,
requires a separate pre-created packet and uses exclusive facts creation.
`--packet-sha256` checks that packet's raw seal and exact membership and requires
`--verify`. A future source candidate needs a new identity, not relaxed old pins.

After first publication, `final-check.ps1` records formatting, eight new and twenty
prior offline controls, reconstructed facts and structural/protected-source
checks, without repeating either allocation probe. Existing output directories
refuse replacement. This is not the full milestone or real fault/reconnect gate.
