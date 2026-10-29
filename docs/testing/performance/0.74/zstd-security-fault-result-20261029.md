# Remaining Zstd public security controls

The selected security subgate passes on Windows/GNU Rust 1.94 and MSRV 1.88.
This closes the previously untested seekable/legacy behaviors for these exact
upstream versions and fixtures. **Root adoption, Chitchat MTU/allocation and the
product workspace/security gate remain open.** No release/server admission follows.

## Fixed baseline, source and method

This follows the [fixed plan](zstd-security-fault-plan-20261029.md) and
[method](../../../../tools/zstd-security-fault-074/README.md).
Root remains at the [first changed-candidate refusal](windows-markdown-workspace-result-20261029.md):
three Zstd findings, not a complete workspace pass. No unchanged full gate was
rerun. Native tests used uncommitted diagnostic sources over baseline HEAD
`27d51d666979b56d4790d210a404c05384abf281`; exact working bytes and producer
versions are retained, not represented as a new product-candidate identity.

Use the already reviewed official zstd 0.14.3 / zstd-safe 8.2.0 archives:

- zstd: `db3ae9bf90af6e40197bef767eeefc46ad6397420fe873830bd85e5fbbfb3c70`;
- zstd-safe: `ded520f16b29e1a9b771c722221b2b0a0398c2316d9bb3c42b458a5414d7434b`.

The safe library fixture preserves all original Rust bodies and 18 canonical
members excluding its local fixture lock. Its only manifest overlay is an empty
workspace table isolating the source from the product workspace. Packaged lock
bytes remain separately labeled; fixture locks are local resolved inputs.
The probe and upstream locks have 15 and 33 records, with 34 checksum-verified
registry archives in their union. Private Windows and nonroot GNU vendors are
checked against those archives before and after commands. The new upstream
packages declare BSD-3-Clause; the existing root license policy permits it.

No old known-unsafe seekable/legacy function is executed to manufacture a paired
exploit baseline. Earlier source evidence and the first borrowed-context failure
remain unchanged. This step therefore claims no reproduced old native heap leak
or paired exploit observation.

## Actual checks

The controls address the primary advisory contracts:

- [RUSTSEC-2026-0391](https://raw.githubusercontent.com/RustSec/advisory-db/40f5be6fb12ed93fc591bf4ab23b7d793b3f2239/crates/zstd/RUSTSEC-2026-0391.md):
  abandon encoder jobs at four sizes and partially read decoder jobs at three
  sizes, then reuse the original contexts. The next decoded output must contain
  exactly the second job.
- [RUSTSEC-2026-0398](https://raw.githubusercontent.com/RustSec/advisory-db/40f5be6fb12ed93fc591bf4ab23b7d793b3f2239/crates/zstd-safe/RUSTSEC-2026-0398.md):
  exercise both buffer and callback-backed seekable readers. Eight offsets,
  including end/past-end and u64::MAX, cross four capacities including zero.
  Written count, Vec length, capacity and exact payload slice must agree.
- [RUSTSEC-2026-0403](https://raw.githubusercontent.com/RustSec/advisory-db/40f5be6fb12ed93fc591bf4ab23b7d793b3f2239/crates/zstd-safe/RUSTSEC-2026-0403.md):
  use the exact six-byte corrupt v0.7 frame from upstream. Failure must leave
  input/output positions and the Vec length unchanged. A poisoned context rejects
  new input until explicit session reset. Write/flush/normal drop/auto-flush/
  on-flush and all seven split points must expose no bytes to the sink.

Five valid modern payload sizes additionally roundtrip through ordinary and
write decoders. These are bounded correctness assertions, not timing/CPU/RSS or
arbitrary unsafe-memory proof. Exact flush error codes are not qualified by the
sink-exposure assertions.

| Peer | Public controls | Original upstream library cases | Ignored / filtered | Offset rows |
| --- | ---: | ---: | --- | ---: |
| Windows 1.94 | 8 pass | 20 pass | 0 / 0 | 64 |
| Windows 1.88 | 8 pass | 20 pass | 0 / 0 | 64 |
| GNU 1.94 | 8 pass | 20 pass | 0 / 0 | 64 |
| GNU 1.88 | 8 pass | 20 pass | 0 / 0 | 64 |

All actual catalogs and ordered offset diagnostics are identical. Probe
all-target/all-feature check and strict Clippy pass on every peer. The exact
upstream library's selected std/seekable/default profile also passes check and
strict Clippy on every peer. This is **112 native test executions**, not 112
distinct functions or full upstream integration/all-optional-feature coverage.
Owned Rust formatting passes. Fourteen final inert controls pass; earlier
8- and 12-control receipts remain separately retained, not extra unique tests.

Cargo artifacts and executable hashes, complete native listings, unfiltered test
summaries, fixed argv, full fixture/source/lock/registry and HEAD postflights are
bound. Actual compiled features confirm legacy and seekable in both safe/sys
libraries. bindgen, cmake, Rust allocator routing and zstdmt are not compiled.
Downloading their optional dependency sources does not qualify those branches.

## First refusals and policy

The first offline upstream metadata command refused missing bindgen index data
before compilation. A separate source-supply phase fetched prerequisites without
changing dependency feature declarations or shortening the tests. That first
refusal and original source/command remain immutable.

The first probe policy refused **this owned diagnostic package's absent license**.
Upstream's standalone policy passed. A new license-only diagnostic fixture declares
the actual project license Apache-2.0, keeps the old lock byte-exact and addresses
the same Rust file. Both that successor and upstream pass advisories, bans,
licenses and sources with byte-exact root deny.toml, offline/locked/all-features,
and no target/dev exclusions or new exceptions. The licensed fixture is a
policy-only metadata successor; no new binary/runtime qualification is inferred
from changing its package path or license field. Native tests were not repeated.

Advisory DB pre/post observations are retained. They describe the observed
repository, not a seal on every internal cargo-deny lookup. A green standalone
fixture policy cannot stand in for the refused product graph or complete
transitive safety certification.

## Limits and next boundary

Original owned review limits remain 1,200 seconds, nested sessions 1,100 seconds,
physical/native floors 4 GiB and polling/cleanup five seconds. Setup's internal
60-second formatting and 300-second metadata/fetch limits do not extend that
envelope. All existing production live checks and 300/60/10, test 5/10/45,
package/workspace 780/4800 contracts remain unchanged.

Root lock remains
`34d7a21d9833a180460e5a094f9ee7f1effe07dfb43a65d90865798963e0070c`;
deny policy remains
`16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930`;
qualification manifest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
No existing service, supervisor, clock, server, product workload or publication
was touched. No allocation/CPU/throughput or retained-memory claim is made.

The [S preview allocation decline](chitchat-preview-cost-result-20261028.md)
still holds: 71/239 cases add allocations. The conservative M bound still loses
nine valid admissions. These results cannot be waived for a green security
fixture. A compatible MTU/backport implementation, source distribution/public
consumers and real product/GNU/package/workspace gates remain required, alongside
original delivery, all-writer retirement, durable lifecycle/reconnect, enrollment
and host rehearsal. No replacement performance hypothesis was opened here.

Raw sources/archives, exact fixtures, producer versions, all first streams,
receipts and independent comparisons are retained in
[`local-runs/zstd-security-fault-20261029/index.json`](local-runs/zstd-security-fault-20261029/index.json).
Native binaries and build/vendor caches are excluded; original pre-format Rust
bytes are retained too.
The packet contains 4,352 logical members / 3,881 objects; index SHA-256:
`f6e65d045c764e113055b24fdcf3ffc180c634931fc1472d9d9d54262054e13e`.
Its first offline retainer refused because it incorrectly expected every negative
CLI exit to be 1: the saved cargo-deny license refusal returned 4. The successor
accepts that exact already recorded failure code, still requires retirement and
no cleanup errors, and retains the first retainer/exception. No native test or
policy command was repeated to repair this accounting error. Both accepted
standalone policy DB observations were clean at
`40f5be6fb12ed93fc591bf4ab23b7d793b3f2239` before and after.
