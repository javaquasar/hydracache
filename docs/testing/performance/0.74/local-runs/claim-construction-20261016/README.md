# Campaign claim construction ownership

The pending owner corrects a reachable flock-lifetime defect before a campaign
claim exists. Errors and active recovery returning None release the original
successful lock even while an owned pre-exec child retains its descriptor.
Success transfers the same File into the unchanged HostExecutionClaim without
unlocking at constructor return. This does not authenticate original-start or
prove writer cessation, and does not identify either old natural Host(Busy) owner.

Source and tree are pinned in [manifest.json](manifest.json), with baseline
eaebeb8f and [preregistration](../../diagnostic-claim-construction-plan-20261016.md)
79b3de2a. The successful claim's fields/Drop, terminal release, short-fence owner,
marker format, authority/epoch/kernel/time checks and qualification inputs remain
unchanged. Only the two construction lifetimes are corrected.

## Causal controls and retained negatives

Six new Linux tests reuse the exact previous private owned-fork/pipe/pidfd
fixture. One-shot thread-local checkpoints run only in tests after successful
acquisition at each campaign site; production has no new callback or factory.
The child stays before exec and retains the original description with CLOEXEC.
Readiness is acknowledged by pipes, never by a sleep or changed signed deadline.

The baseline run passes the successful-transfer group on all three routes but
fails five groups at their first independent lock probe: named acquire, named
recover, active recovery errors, active recovery None and late refusal after
marker creation. Each fails WouldBlock while the old alias is still retained.
Later cases in those failing loops are not baseline observations.

The baseline runtime plus test checkpoints/module registration is recorded in
baseline-overlay.patch, with the new tests in baseline-tests.rs.txt. This
snapshot was taken after formatting; the initial red run compiled the preceding
format and its source positions differ. The development green precedes final
borrow/format completion. Neither receipt is an exact binary or clean-build
attestation. Final source-pinned host-unit logs are separate.

After correction, all six tests execute 23 enumerated cases: eleven named
acquire/recover errors, four active errors, two None outcomes, three late refusals
and three successful transfers. Existing marker bytes/inode/mode are preserved;
a newly created marker survives late refusal. Independent next-owner probes
remain locked across old-child late unlock/close and failed contenders. Successful
claims retain ownership until the existing claim Drop.

PendingHostClaim owns Option<File> only after successful flock and before
fallible admission. Consuming transfer takes the original File and leaves None
so temporary Drop cannot unlock the successful claim. Early-return Drop explicitly
unlocks; this is best effort, not a durable release receipt. No clone, unsafe
extraction, reopen, relock, retry, marker removal, public API or protocol change
is introduced. Exact-source restoration and negative canaries preserve every
other runtime statement and the prior static assertions.

## Final local matrix

| Check | Rust 1.94 | Rust 1.88 |
| --- | ---: | ---: |
| Linux host unit group | 19 pass | 19 pass |
| Linux fixed scan / preparation / proof reuse | 4 / 14 / 5 pass | 4 / 14 / 5 pass |
| Linux IPC / lease / original fence / host integrations | 24 / 20 / 5 / 6 pass | 24 / 20 / 5 / 6 pass |
| Linux ordinary original-input regression | 99 pass, 4 ignored | 99 pass, 4 ignored |
| Windows portable IPC / lease / host integrations | 21 / 18 / 5 pass | 21 / 18 / 5 pass |
| Linux and Windows affected-package check + strict Clippy | all pass | all pass |

Four ignored original-input tests are the existing registered manual fixtures,
not new exclusions. The original-input invocations follow completion of this
matrix's builds/formatting/other runtime tests and retain ordinary internal test
parallelism. No old integration failure is retried or old test serialized.

The complete static transcript passes 179 contracts. Expected caught-panic
canaries and the old IPC injected-loss control print panic text under nocapture
without failed tests. Formatting, docs, all 176 discovered gated-test
registrations and local non-promotable performance contract pass. Ship admission
fails as expected. Four final documentation/registry/contract receipts follow
the documentation edits.

No product workload or performance screen, server or GitHub Actions run, new
SIGKILL matrix, full release verify, separate kernel/manual causal/compile-fail
suite or push is part of this slice. Prior timing evidence stays pinned to its
historical source; no gain for this composed source is claimed.

## Verify without execution

Run verify.ps1 from this directory, the repository or its parent. It checks
source/tree and preregistration ancestry, the seven-path baseline delta, six
selected source files, frozen inputs, command outcomes/counts, all 23 causal
case markers and packet artifact inventory/hash/length. It builds or launches
nothing and changes no files. Unsigned transcripts and self-hashed local evidence
detect drift relative to the manifest, not independent execution attestation.

The next execution work is authenticated original-start and autonomous
revocation/cessation of every writer, then completion/later cells and faults on
their new publication boundaries. This construction correction does not open
those lifecycle gates or dedicated-host admission.
