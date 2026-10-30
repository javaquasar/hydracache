# Direct supervisor Zstd security update

The unpublished supervisor now pins `zstd = "=0.14.3"`, resolving
`zstd-safe8.2.0`. Its fresh advisory-only check passes. The complete Root check
still refuses exactly three advisories through Chitchat0.11.1. **This is a
private-tool security update, not acceptance of Q, a full package/workspace pass,
a performance result or permission to execute a product campaign.**

## Scope and exact input review

Implementation commit: `81149cb02`. Only
`tools/long-run-supervisor-074/Cargo.toml` and `Cargo.lock` change there.
The exact historical lock is
`34d7a21d9833a180460e5a094f9ee7f1effe07dfb43a65d90865798963e0070c`;
the accepted direct-edge lock is
`8ade63f6a4aae93f27f5161798bc87804542a48093b4203f35e25e29b680a4f5`.
There are683/685 locked nodes. Two registry nodes are added; the only changed
consumer is the unpublished supervisor. Old Chitchat→Zstd0.13.3→safe7.2.4 stays.
Version disambiguation does not change that old edge. All unrelated historical
blocks retain their original bytes and order. The first semantically valid
candidate unnecessarily reordered unrelated versions; its bytes and successful
metadata result remain retained, followed by the exact-order successor.

Official archive identities remain the previously reviewed
[Zstd fault-package inputs](zstd-security-fault-result-20261029.md):

- zstd0.14.3: `db3ae9bf90af6e40197bef767eeefc46ad6397420fe873830bd85e5fbbfb3c70`;
- zstd-safe8.2.0: `ded520f16b29e1a9b771c722221b2b0a0398c2316d9bb3c42b458a5414d7434b`.

The existing zstd-sys2.1.2+zstd1.5.7 node is unchanged. No dependency exception,
MSRV increase, disabled feature or new product optimization accompanies this
update. `deny.toml` remains
`16ee708c678497358f30d772cbb843832b4e02e8f3ae4355b648620a94c44930`;
the qualification manifest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.

Fresh source/effect review finds the same single archive encoder construction,
`zstd::Encoder::new(output, 19)`, in `src/archive.rs`. This consumer does not
reuse borrowed contexts. Its limited writer, deterministic TAR headers, input
inventory, output placement, sealing, digest verification and original bounds
are unchanged. No runtime owner/manager/clock/watchdog predicate or call count
changes. The [upstream advisories](https://rustsec.org/advisories/RUSTSEC-2026-0391.html),
[seekable correction](https://rustsec.org/advisories/RUSTSEC-2026-0398.html) and
[legacy-stream correction](https://rustsec.org/advisories/RUSTSEC-2026-0403.html)
explain why the wrapper upgrade is still appropriate; absence of a particular
call at this consumer is not a policy exemption for the remaining old graph.

## Actual checks and their limits

Four unchanged archive cases pass on the historical Root baseline before the
dependency mutation. On the new exact Root graph, the complete same four-case
catalog passes on Windows/MSVC and GNU/Linux, each on Rust1.94.0 and MSRV1.88.0:
**16 candidate case executions**, plus the four baseline executions. They cover
deterministic archive bytes/headers, digest and extra-file drift, create-new/
size/output-placement refusal, and hardlinked-input refusal. No case is ignored;
each exact invocation filters the other three, with the complete catalog
accounted once for every peer. These are targeted archive tests, not the90-target
full supervisor test catalog.

All four platform/compiler peers pass all-target/all-feature `cargo check` and
strict `cargo clippy ... -- -D warnings`. Full GNU workspace formatting passes.
The initial full Windows formatter refuses with OS206 (command length); its
output is retained. The separately bounded affected-package Windows formatter
passes. This does not convert the failed full Windows invocation into a pass.

A separate test-only fixture exercises actual archive creation with sizes
0/1/4095/16384/131072/262144, seed7606312, duplicated inputs and both decoder
versions. Its six complete observations match between the old/new direct
consumer and Windows1.94/1.88; both decoders read every produced frame. **That
fixture has its own lock, newer helper packages and old safe7.3.0, whereas Root
has safe7.2.4.** It is additional compatibility evidence, not an exact-Root
byte-equivalence or performance claim. GNU probe runs are not represented as
completed. Root regression evidence above uses the actual locked Root graph.

The new all-feature resolve has36 members and23 publishable roots. Their full
reachable closure contains599 nodes and reaches neither the supervisor nor the
two new wrapper versions. This proves the direct update's private reachability
boundary; it is not a registry-consumer test or publication authorization.

All affected Cargo phases remain offline/locked. Original Windows Job/Linux
Group retirement, four-GiB physical/native floors, five-second polls/cleanup,
local1200/native-session1100 envelopes, and five-second case/ten-second catalog
bounds remain. Nothing is installed; no existing service, clock or dedicated
server is touched. The full780/4800 package/workspace gates are not relabelled
or replaced by these focused checks.

## Preserved first refusals and preparation repairs

The first fixture resolves against nonexistent suffixed D-Bus paths and refuses.
Those paths are corrected in a separately captured context. A one-test catalog
then exposes the parser's plural-only expectation; the original successful
listing is accounted offline with the exact singular grammar, without relisting.
The first candidate fixture build refuses `--locked`: its separate supervisor
edge still names old Zstd. Only that edge changes in its fixture lock; the old
fixture lock and refusal remain. The corrected Windows peers pass.

The first GNU preflight cannot interpret the Windows worktree `.git` path. A
separate explicit native-Git address works, but native `git status` then spends
several minutes refreshing the Windows worktree. Read-only process attribution
identifies that exact operation before Cargo. Its own Git child is cancelled
through a held pidfd after exact UID/parent/cmdline/start-ticks checks; this
cancelled context remains negative. No service or foreign process is signalled.
The successor performs the same Git reads using Windows Git, without removing
the checks. Its first Cargo attempt then refuses missing pulldown-cmark0.13.5
in the older offline cache. That already approved source is supplied before the
final complete GNU context; no Root dependency is changed for that repair.

Only the two reviewed Zstd versions and the missing approved markdown version
are added to the non-product GNU cache. All original644 source inventories,
receipt and config stay unchanged. Fresh full647-package source/config checks
surround every final native phase; the unused old markdown cache entry remains.
The separate fixture's newer helper graph is not silently substituted into Root.

## Fresh policy result and the Q decision

Private RustSec snapshot:
`ee1a49a2769a24a596afc1c20cb33748d2769586`. The complete original policy is
copied with only advisory storage location changed. Fetch is disabled for both
actual checks; DB/source/config pre/postflight is exact. The supervisor has zero
advisory errors; its five unmatched existing-ignore warnings remain warnings.
Root refuses RUSTSEC-2026-0391,0398,0403. Each remaining reverse graph goes
through Chitchat and none through the supervisor. This is the default configured
advisory scope, not a new all-transitive-unsoundness qualification.

The [complete Q CPU/RSS matrix](chitchat-cohort-r3-result-20261030.md) remains the
decision input:11/62 frozen CPU/wall upper bounds exceed1.03. Q is unadopted;
its source, negative decision, all first failures and complete data remain in
the feature branch and main release documentation. The successful private-edge
update does not override Q's CPU or general stateful-resource refusals.

Next resolve the remaining Chitchat/security/distribution boundary without
promoting Q or changing thresholds. Then complete the current exact-input full
package/workspace gate. Production authenticated original delivery, all-writer
cessation, lifecycle/crash/lost-ACK, same-attempt reconnect/retention and actual
trust/enrollment/compatible-host rehearsal remain separate open gates. No C74
freeze, product qualification or server-launch admission follows here.

## Retained evidence

[Primary data-only packet](local-runs/supervisor-zstd-security-20261030/index.json):
SHA-256 `01b16a601a6346b4656d3f258e7f48da1949e4532d8445fdd653aefc219f91d3`,
1863 members/1630 objects. It retains every first refusal, corrected phase,
catalog/result/ownership stream, both Root input bodies, separate fixture locks,
official input archives, exact advisory snapshot and current producers. Compiled
images and full native cache copies are excluded. Inherited environment values
are not exported: sanitized intent copies retain only fixed compiler settings;
the index records hashes of their private originals and exported copies.

The separate final-check packet,
`local-runs/supervisor-zstd-security-20261030-verification/index.json`,
retains subsequent offline verification, mutation/refusal controls, syntax,
documentation and non-promotable performance-contract checks without rewriting
the primary packet. Reproduction and scope are documented with the
[current verifier](../../../../tools/supervisor-zstd-policy-074/README.md).
