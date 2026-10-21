# Bounded Groups collection allocation evidence

This packet retains a baseline followed by one collection-only candidate.
Both compiler lanes use seven lengths and 256 collections per length; these
are pure allocator units, not fresh host profiles or original-owner attempts.
The source commits in `identity.json` reproduce the method and candidate.

`baseline/` and `candidate/` keep the complete first check logs and outcomes;
`controls/` contains the four additional ordinary pure parser controls per
compiler. The original `kernel_projection` selector matched zero tests. Its
log is retained and is not counted as coverage; the four explicit selectors
provide the intended parser coverage without changing the initial recipe.
`method/` retains all three recipes. `final/` keeps formatting and local
documentation, registry and performance-contract checks. `source/` archives
the baseline and candidate source/control files at their exact commits.

After sealing, run from the repository root:

```powershell
pwsh -NoProfile -File tools/local-profile-session-074/review_projection_groups.ps1 -Mode Review
```

Review reads logs and hashes, checks source identity and prior raw packet
membership, and recomputes allocator totals. It runs no Rust/WSL test, clock
sample, live owner, service, server or workload. Like earlier packets, it refuses
future runtime/source changes instead of accepting arbitrary successor edits.
The inventory seals every member except itself. Never replace a member or seal.

The result establishes fewer Rust byte requests and reallocations in the
bounded collection only. It does not measure exclusive CPU, retained memory,
RSS, validation latency or product throughput. Original windows 300/60/10 and
the two first Causal(Time) failures remain unchanged; readiness stays closed.
