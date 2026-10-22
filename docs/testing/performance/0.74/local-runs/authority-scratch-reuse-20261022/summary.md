# Scratch reference and storage-only candidate

Read `facts.json` with `tools/authority-scratch-reuse-074/review_reuse.py`.
This packet separates reference-only preflight, first candidate measurements,
first tooling refusal, repaired host checks and final local checks. Source
snapshots and identities retain the distinction; no measured cohort was rerun.
See `development-note.md` for first runner/canary/test-fixture failures and repairs.

Both compiler lanes: 2304 full revalidations per arm. Reference: 2048 allocations,
1222656 requested bytes. Cold scratch: eight allocations, 4776 requested bytes;
cache drop separately releases eight allocations of 4776 bytes. Fresh-read
outcomes/traces match in the paired synthetic controls. Extra retained body
capacity and a 32-byte wrapper remain costs. No physical copy/RSS observation.

Decision: retain-unpromoted-prototype. Timing is preliminary under concurrent
host activity, not an isolated/full-path speedup. Prior eighteen raw packet seals
and 1000 members preserve first original failures and windows. No WSL/live/server
run, runtime integration, clock/service change or product workload. All original
material/all-writer/full-fault/reconnect/server admission gates remain closed.
