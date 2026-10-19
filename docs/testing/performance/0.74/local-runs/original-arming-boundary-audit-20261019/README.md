# Original arming boundary audit checks

This packet records regression checks of source
`1fa0a920fe09919f235cdc0f684fd0e281d2c3f9` while auditing the missing original
Pending to Armed transition. It is not a new implementation, performance
measurement, target enrollment or server readiness result.

The [boundary audit](../../original-arming-boundary-audit-20261019.md) identifies
the direct-child versus systemd target mismatch and the missing pre-delivery
revocation root. Its proposed dedicated-root topology requires explicit review;
no topology, protocol or live-check change is selected here.

Raw logs are retained alongside this file. Unit controls exercise owned local
kernel fixtures only. No original/system-bus send, release installation,
supervisor operation, server access or product workload is part of these checks.

## Results and reproduction

- Windows `cargo test --locked -p hydracache-long-run-supervisor-074 --lib --
  --test-threads=1`: 49 passed. The matching `--doc` invocation passes 17
  compile-fail controls.
- `cargo test --locked -p xtask --test performance_contract_074
  original_pending_watchdog -- --test-threads=1`: 3 passed, 271 filtered out.
  This is the focused source subset, not a rerun of all 274 controls.
- Each retained GNU current/MSRV libtest image run with
  `diagnostic_two_phase_launch:: --nocapture --test-threads=1`: 22 passed,
  3 ignored. The ignored process harnesses were not run by this filter; their
  earlier receipts remain in the previous packets. No new full-frame,
  process-loss, SIGKILL or lost-ack matrix is claimed here.
- The corrected Pending inventory verifier with `-SelfTest`: 94 files,
  8 inventory mutations and 6 prefix/encoding controls pass. Its append-only
  historical audit with `-Verify` binds 4 packets and 182 unchanged files.
- `cargo run --locked -p xtask -- doc-check`, `gated-test-check` and
  `performance-contract-check --release 0.74` pass. The registry still discovers
  184 gates; the local performance contract is non-promotable. Scoped
  `cargo fmt -p hydracache-long-run-supervisor-074 -p xtask -- --check` passes.

No Rust or configuration change was made, so this packet does not repeat
compilation or strict lint and does not claim a new all-source gate run.

The GNU image paths and before/after SHA256 values are retained verbatim in
`gnu194-image-before.log`, `gnu194-image-after.log`, `gnu188-image-before.log`
and `gnu188-image-after.log`. They match the selected source/image receipts in
the previous Pending packet. No image is rebuilt for this documentation-only
audit. `source-unchanged.log` records an empty diff against the baseline for the
entire supervisor, xtask, compatibility, registry and qualification inputs.

Local WSL `systemd-timesyncd.service` is active. This audit never stopped it,
created an admission origin or changed a clock. No elapsed unit-test time is a
product performance claim. `inventory.json` binds the retained packet's exact
raw bytes, including line endings, without granting runtime authority.

The per-packet `.gitattributes` preserves those raw bytes on add and checkout.
To verify the inventory, compare its complete relative file set (excluding
`inventory.json` itself), each `bytes` value from `Get-Item` and each lowercase
SHA256 value from `Get-FileHash -Algorithm SHA256`. No text-prefix removal or
newline canonicalization is used. The inventory is a local checksum list, not
a signature or production trust/enrollment artifact.
