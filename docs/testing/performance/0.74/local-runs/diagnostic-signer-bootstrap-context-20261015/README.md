# Structured signer bootstrap context local evidence

Implementation `7523ce27a12d3088b75c66fa39e82da90b017182`, tree
`e388f5a88f0ee314535101895f58c3be7b9eab7c`; preregistration
`f4566f7da419b120d965e91f5cee52347a996e92`; baseline
`8aa8e208128c0db93084e6fea1fe53a3e84c9258`.

The optional stronger portable/Linux APIs check the old receipt's bootstrap
context as a canonical structured statement. Source/binary/runtime bindings,
bounded separate launcher assertion, fixed job profile and six false capability
declarations must match. Old opaque model/runtime APIs remain unchanged. The new
result cannot extract a weaker guard or authorize signing/start.

## Checks and falsification

Before mutation, portable build/GNU tests passed 9/6 and Linux original-runtime
composition passed 8. The new portable API failed E0432 before implementation.
Seven new portable cases pass repeated Rust 1.94 Windows/Linux runs and both
Rust 1.88 MSRV runs. They cover every field, correctly re-signed unsafe contexts,
canonical/unknown/duplicate/type/budget faults, wrong signature/key/content,
64 signed schema mutations from `0x0740c078` and four independent models.

The Linux original-runtime corpus now passes 9 cases, including legacy opaque
success, stronger opaque refusal before acquisition, synthetic valid composition,
public nonroot-fixture refusal and sticky original failure. OIDC regression passes
18; root-owned inventory 13; old build/GNU/issuer 9/6/6 on both platforms. Three
portable compile-fail cases and two Linux wrapper compile-fail cases pass. The
full root suite passes 155 contracts. Current all-target checks/strict all-feature
Clippy pass for the supervisor on both platforms and xtask on Windows. Supervisor
all-target MSRV checks pass on both platforms. Scope is focused local verification,
not full workspace milestone verification or release qualification.

Scoped format, doc-check, all 172 registered gates and local non-promotable
performance-contract-check pass. `--require-ship` remains expected-red exit 1.
No threshold, existing signed domain, old job profile value, dependency, lock,
qualification manifest, actual trust configuration or workflow was changed.

A deliberate limitation is tested: a different well-formed launcher/workflow
assertion refuses under the original expected statement, but may pass after
explicitly selecting a different complete expected statement and synthetic
receipt. This is not independent policy enrollment. Portable positives deliberately
retain non-ELF/opaque-runtime inputs; only the separate original GNU path parses
runtime declarations. Linux positives use a cfg(test) nonroot root seam, not
production ancestry. The actual launcher is never acquired or executed.

## Replay and admission boundary

Run `python -B verify_packet.py` and `python -B test_verify_packet.py` here, or
`python3` on Linux. The manifest hashes normalized-LF captured tool output, file
lengths and observed outer exits, implementation source files and frozen inputs.
Replay reads packet files and Git objects, not real keys, runtime inputs or network
services. The packet is unsigned local evidence, not execution attestation;
rehashing observations cannot authenticate their origin. Tests were developed in
the working tree; the implementation commit records their final source, not a
claim that every earlier baseline/red run used that final tree.

No actual key/token, production receipt, job protection, launcher/root custody,
GitHub/server mutation or workload was involved. Shared old OIDC constants retain
their previous values; its configurable subject behavior remains compatible.
Independent original launcher/runtime/job custody and actual receipt remain
before bootstrap admission. Worker original-start, every P2/P3 writer's cessation,
completion, cells 2–4 and server/pilot/qualification admission remain closed.
