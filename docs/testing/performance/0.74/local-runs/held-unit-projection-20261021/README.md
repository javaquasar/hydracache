# Consumed owner held unit projection checks

This packet covers the semantic projection described in the
[implementation report](../../held-unit-projection-20261021.md), based on
`392fa72188f84ca348e9f21c3e689d6546f48afb`. The new wrapper consumes and retains
the original joined holder, offers only revalidation and cannot send, release
or export executable/descriptor/unit material. Missing launcher FD objects are
explicit requirements, not materialized kernel evidence.

`checks.ps1` records three independent local lanes: ordinary-user WSL GNU 1.94,
WSL MSRV 1.88, and Windows source/repository checks. Each compiler selects 10
new projection controls, 10 unchanged consuming-holder controls, 12 unchanged
reply state/flag/exchange controls and 38 synthetic build/role controls: 70
selected tests per compiler. Check and strict clippy
cover all targets; clippy enables all features. The host lane adds nine source
canaries, including exact reconstruction of 462 historical files, affected
package checks/lint, formatting, docs, gated registry and the local
non-promotable performance contract.

Before/after manifests bind every supervisor source/test, xtask source/test,
Cargo.lock, COMPAT, registry and qualification file. `review.ps1` checks equal
contexts, exact selected counts, command/log hashes, unchanged qualification,
the preceding immutable 41-member audit packet and bound reports. It does not
rerun an old verifier against changed runtime or rewrite its sealed claims.
The new exact source adapter removes four hooks only; it cannot hide changes
to an old full-frame, transport or retirement body.

`binding.json` identifies the committed implementation and report digests;
`inventory.json` covers every other packet member without text normalization.
Review is read-only:

```powershell
pwsh -NoProfile -File docs/testing/performance/0.74/local-runs/held-unit-projection-20261021/review.ps1 -Mode Review
```

These are preparation-only unit/property/synthetic and source checks, not an
original live fixture, all-writer proof, SIGKILL/lost-ACK matrix or server
admission. The 300/60/10-second windows remain. No clock, service, installation,
supervisor, server, manager send or product workload is changed or run. No
throughput, CPU, allocation or latency measurement is collected.

The first seal review refused a documentation count mismatch: the broad
`dispatch_reply` selector runs the ten state/flag tests plus two exchange tests,
not ten tests total. The raw 12-test successful logs are unchanged. The
pre-seal count correction changes no selector, test, runtime, deadline or
acceptance predicate; no test workload was repeated to obtain a pass.
