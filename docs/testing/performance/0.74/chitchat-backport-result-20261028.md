# Local Chitchat security backport preparation

This continues the [first feasibility review](remaining-security-feasibility-20261028.md)
after the operator requested continuation. It prepares local source patches;
root dependencies, deny.toml, qualification manifest and Rust 1.88 remain exact.
No package is published/adopted, and no service, clock, server, installation,
product load or expensive qualification is started. Original live predicates,
300/60/10 origins/windows and all package/workspace limits remain unchanged.

## Baseline and isolated source steps

The checksum-reviewed chitchat 0.11.1 archive is the base. New private tests are
added identically to standalone target fixtures; their complete source/lock and
external overlay hashes are checked before/after every command. Each source/
compiler gets a different content-derived target root. The actual Cargo
compiler-artifact executable and SHA are bound before a full native test listing.
Selected identities, pass/fail/ignored/filtered counts and postflight must match.
The original 105-test catalog is preserved. A versioned extension adds exactly
one named oversized regression, with three cases, yielding 106 tests.

| Input | Isolated change | First result / conclusion |
| --- | --- | --- |
| B | Original source/requirements plus shared observation tests | Minimal bound test fails: threshold 1 and string `!` produce 13 bytes against estimate 10. |
| M | Count metadata for all blocks | All 37 stream bounds hold and bytes agree, but nine supported DeltaSerializer admissions become stricter. Not accepted as behavior-compatible. |
| L | Two explicit lifetime annotations only | Check/strict Clippy pass; stream and operation observations equal M. |
| Z | Zstd requirement 0.14.3 only | Original wide snapshot, seven malformed cases and both context controls pass; unchanged observations versus L. |
| R/G | Owned preview on the more-than-two-block branch | Prototype has an oversized-input regression: preview reaches append's u16 assert before the caller can reject. G retains this failure with the extra regression. |
| R2 | Avoid preview for structurally oversized items | Original rejection/writer state preserved; all 239 supported admissions and wire bytes again equal B. |
| S | LRU requirement/locked node 0.18.2 only | Same functional observations as R2, with passing standalone dependency policy and Rust 1.88 controls. |

These are separate frozen inputs, not one combined benchmark hypothesis. The
original broad failures and earlier feasibility records are unchanged. No
threshold is reduced, corpus shortened, lint suppressed or failure overwritten.

## MTU contract and compatibility

The old estimate reserves at most two block headers. At threshold 1 the
serialized three-byte string representation needs three blocks: the original
10-byte estimate is below the actual 13-byte stream. The baseline minimal test
prints both values before failing. Its exact wire is retained.

The deterministic original 37-stream matrix has two baseline bound failures:
16/100/0 and 16/100/1. M fixes the bounds without changing stream bytes. Actual
DeltaSerializer tests then expose why that alone is insufficient. Across 239
supported operations with MTU 100–65,507, nine old admissions are lost despite
their real compressed frames fitting. For example, at MTU 40,000 and value length
39,938, the estimate changes 39,998→40,001 while the old frame is only 33,316
bytes. The complete nine-case counterexample set stays in evidence. This is a
functional observation, not throughput or capacity measurement.

R2 keeps the cheap previous estimate for at most two pending blocks. For a
serializable item spanning more blocks it previews finish on an owned writer
copy and returns max(previous estimate, actual preview length). It does not
change the original writer/builder during prediction. Structurally oversized
items remain rejectable before append, rather than panicking in preview. Tests
retain node-only wire bytes after those rejections. Positive block thresholds
are the supported domain; actual DeltaSerializer still requires MTU>=100.

Final S holds all stream bounds, reproduces B's exact admission decisions/wire
bytes for all 239 supported cases, and passes the unchanged original wide gate.
The seven malformed diagnostics also agree with Z. This finite set does not prove
every possible protocol input, native memory safety or whole-system performance.

## LRU finding and policy scope

The first standalone R2 policy command refuses
[RUSTSEC-2026-0253](https://rustsec.org/advisories/RUSTSEC-2026-0253.html) on
LRU 0.17.0. Under the documented
[workspace scope for unsound advisories](https://embarkstudios.github.io/cargo-deny/checks/advisories/cfg.html#unsound),
making Chitchat a workspace root changes which direct dependencies are reported.
The previous product gate's six findings were not a complete transitive safety
certification. The unchanged product graph still contains old LRU; its absence
from that old finding list is not an exemption or a fix.

S changes only the LRU locked node/requirement to the minimum patched 0.18.2,
with primary checksum verification and unchanged existing hashbrown identity.
It declares Rust 1.85 and has no build script. The retained full source diff
includes detaching a popped node before a potentially panicking key Drop,
a lifetime correction and an added unused lookup API. Original Chitchat state
tests run on both dependency inputs; no native unsafe old-key exploit is claimed.

The S standalone command uses the byte-exact root deny policy, offline/locked,
all root features and no target/dev/unpublished exclusions. Advisories, bans,
licenses and sources pass. Advisory DB head 99670cfc0bc574433034b75876a4b791ccaa2efb
is clean and equal in pre/post observations; it is not represented as a sealed
per-lookup input. The root product gate is not replayed or declared green.

## Checks and retained source

S passes 63 selected native Rust tests on 1.94: 25 original serialization groups,
32 original state tests and six observation/regression functions. On Rust 1.88
the complete test library compiles and 36 selected tests pass. The 37 payload
cases, 239 operation cases and three oversized cases are not extra Cargo target
counts. All-target/all-feature check and strict Clippy pass. Five native-accounting
controls and formatting of the owned Rust test sources pass. Eleven build images
have distinct paths; each selected test is bound to its built image and catalog.
All owned commands retire without cleanup errors under the unchanged 1200-second,
4 GiB and five-second local review contracts.

The [portable patch stack](../../../../tools/chitchat-mtu-review-074/patches/receipt.json)
has four source-only patches: bound/admission repair, lifetime annotations,
Zstd, and LRU. The normalized and authoring manifests mirror the tested dependency
definitions. An independent inert patch replay reconstructs all 21 source member
hashes from the exact base archive, with five changed members and no test-only
paths in the canonical Rust bodies. The retained upstream packaged lock is data;
the separately labeled fixture lock is the tested local graph. This is not a
signed build/enrollment receipt or a published consumer-qualified package.

The [primary packet](local-runs/chitchat-mtu-backport-20261028/index.json) contains
855 logical references to 371 raw objects. Index SHA-256:
`d6fc98b961530fdc3b879018521500c80d65094e7f160263b7318fa8870d5dfe`.
It retains every native command/first failure, successive source patches,
artifact/catalog receipts, differential data and exact tool predecessors.
An initial inert predecessor reconstruction's extra-LF error is retained
separately; its corrected byte proof does not replay any native command. Source
checkout equality for the three new libraries is observed in postflight only.

## Still required before adoption

Subsequent [matched cost attribution](chitchat-preview-cost-result-20261028.md)
now declines S integration: preview adds repeatable Rust allocation work in 71
of 239 cases, including 48 rejected operations. The source-only patch stack and
all prior green functional results remain archived; product adoption has not
occurred. The paragraph below records the original unmeasured boundary at the
time this preparation was completed, not the current allocation verdict.

The preview adds allocation/copy/compression on large-item paths, including some
operations ultimately rejected for MTU. Its CPU/allocation/retained-memory cost
has not been attributed. Native non-regression cannot be inferred from functional
passes. Next compare identical dependency graphs with/without preview, guarding
the same supported inputs, admission and wire bytes; retain any negative result
instead of accepting a more expensive candidate for a green functional gate.

GNU, real HydraCache integration/wire/archive/concurrency checks, distributable
package identity/ownership and complete final package/workspace gates remain open.
The independent zbus Rust 1.88/blocking/wire migration is also unfinished. Original
execution, writer retirement, lifecycle/reconnection and compatible host admission
remain separate release blockers. No numerical performance claim or production/
server readiness is asserted by these local source patches.
