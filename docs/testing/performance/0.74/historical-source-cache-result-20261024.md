# Immutable historical lookup deduplication and ordinary product controls

Test-only candidate `25faa7bb` passes all 321 old source-contract tests with
zero ignored/filtered, seventeen extra current/history/cache/parser controls,
all-target check and strict all-feature Clippy. The source gate's observed
505.35 seconds versus its predecessor's 537.81 seconds are single unpaired
cohorts, with other local work present. They do not establish a repeatable
elapsed-time gain and are not product measurements.

The confirmed mechanical result is narrower: the same immutable key loads once
across 64 sequential lookups and sixteen concurrent callers. Returned strings
are independent. Revision, path and cache-instance separation, nested different
keys and loader failure without publication are tested. The integrated wrapper
accepts full forty-hex commit IDs only; the exact original Git command, manifest
root and LF normalization remain in its unchanged loader body. The duplicate
historical reader delegates to it after proving the same root.

No current file, `accepts` result, policy, credential, procfs, boot/namespace or
clock input is cached. Fresh complete current reads and all mutation assertions
remain. Earlier compiler warnings from a too-broad test-helper import are kept;
the final source-delta control uses its own narrow whole-token comparison.
Standalone five controls and the integrated seven cache controls pass. Cache
memory belongs only to each source-test process and never enters measured native
or product processes; no RSS/allocation reduction is claimed.

Separately, the first ordinary Windows RESP/client-surface correctness command
passes 251 tests across 23 targets; 25 explicitly ignored cases remain ignored,
zero cases are filtered and external opt-in environment is absent. This exercises
codec, translation, protocol, staged order, adversarial transport and direct
client-surface tests, not the external oracle, benchmark or long-run qualification.
Product sources and Cargo.lock remain unchanged from `8fb0fb5a`.

The [27-member packet](local-runs/historical-cache-product-fast-20261024/index.json)
has 26 objects and index SHA256
`95e1015fd87aa172b74ecc36a076dbcc1c56479b584ae3b28b6b10702e068c84`.
It retains the first cache controls/source, exact delta checks, complete cohort,
check/lint, ordinary product logs and coordinators. This does not supersede the
failed ordinary supervisor/MSRV entry gates, original preparation expiry or
unproved delivery/retirement/enrollment boundaries.
