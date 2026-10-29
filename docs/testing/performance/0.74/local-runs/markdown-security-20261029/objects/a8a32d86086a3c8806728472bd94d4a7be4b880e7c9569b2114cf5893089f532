# Pure collector peer vector baseline

Freeze the complete existing peer body: supplementary to_vec, primary push,
sort_unstable and dedup into an owned DiagnosticPeer. The production collector
file and wrapper do not change. Only a private opt-in allocator-test hook is
added; exact source guards reverse that hook without exempting a live body.

`check.ps1 -Lane host|194|188 -Cohort <new-name>` refuses existing logs. Host
runs 34 source canaries plus affected check/strict all-feature lint. Linux runs
five new pure controls, five existing trust-stack controls, four context-stack
controls and two counter controls, then affected check/lint. Ordinary-user WSL
supplies Rust build/units only; no ignored/manual/live fixtures are selected.

The main corpus has 31 cases, 256 constructions each. A separate one-construction
stage control per case attributes Rust requests to clone/push/canonicalization;
do not pool its count with the main corpus. BTreeSet differential checks use
seed 0x074c0ee1 and 1024 cases, with explicit 32-to-33 union/no-truncation and
owned-value checks. Generic oversized inputs are not upstream admission.

The complete synthetic wrapper restores its production tokens after reversing
one peer-observation call. Fake field receivers preserve first-error, latch and
reentry order over 32 multi-fault combinations. No actual KernelPacket, sender,
origin, file or clock is constructed; these are not live fault proofs.

First Linux compiles refuse an ambiguous integer type. First host canaries
refuse a static lifetime token in case-label types, not an actual static object.
Preserve first logs/source. The corrected cohort adds an explicit usize and
uses owned labels outside measurement; no canary or runtime predicate is relaxed.

After the exact source commit, `review.ps1 -Mode Seal -Source <40-hex-commit>`
creates fsynced metadata once; `review.ps1 -Mode Review` checks it offline.
It verifies nine prior raw packet seals/401 members, both cohorts, stage sums
and source/method snapshots. Historical source-pinned reviewers stay unchanged.

This baseline implements no vector candidate and measures no CPU/RSS, retained
memory or preparation-time gain. Original 300/60/10-second windows and both
first Causal refusals remain. No clock/service/server/SSH/Actions/install/
supervisor/product operation, live replay or material/fault/server promotion.
