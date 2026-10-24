# Historical Git mutation controls before caller integration

The isolated native reader is **not admitted for a real caller**. A deterministic
replacement-ref ABA changes an absent path into a present blob between validated
batch input and `show` fallback. The reader returns the new, unpinned bytes even
if the ref is restored before return. Pre-batch stale pins are refused, but that
does not close this later observation gap.

Code commit: `44a3f7b120e352a259279bf9c72e39a74ab22a12`.
The [evidence index](local-runs/historical-blob-fault-20261024/index.json) has
SHA256 `37d771195efc42b8757d32b4b9634fdb114f97d8c13747b7b3ee4a9094a8b3a3`:
633 logical members, 137 content-addressed objects and 19 captured phases.
Each phase retains its first source version, intent, stdout, stderr and result;
mutation observations are written before their assertions. This is a semantic
diagnostic, not CPU, throughput or memory measurement.

## Boundary and implementation

`tools/historical-blob-batch-074/transport.rs` remains imported only by isolated
xtask targets. `read_observed` exposes logical cuts after batch validation and
before/after fallback; ordinary `read` passes an empty observer. The original
validation calls, order, policy flags, budgets and shared deadline are unchanged.
An observer that exhausts that deadline produces `Deadline` before fallback;
it gains no new grace period.

`crates/xtask/tests/historical_blob_fault_074.rs` mutates only disposable test
repositories. `collision.py` creates one deterministic, bounded commit-prefix
collision using seed `hc074-collision`, then Git verifies its object ID. It does
not produce trusted source pins or change the repository under development.
The fault oracle prepares two needed reference profiles and 18 results; default
oracle preparation still selects all six profiles and 54 results. No old loader,
guard or runtime is redirected, and no dependency changes are made.

## Exact outcomes

| Mutation before batch | Expected and observed native refusal |
| --- | --- |
| Blob replacement with the same OID header and body length but different bytes | `Protocol(Content)` |
| Local `core.useReplaceRefs=false` makes the old replacement pin stale | `Protocol(Header)` |
| Commit replacement introduces a path pinned as absent | `Protocol(Header)` |
| A second commit makes the original four-digit revision ambiguous | `Process` |
| The exact pinned loose object is held outside its object-store path | `Protocol(Header)` |

Explicit replacement disabling preserves original bytes/absence in the respective
controls. An unrelated branch and HEAD move does not change a full-ID query.
Restoring the held object restores original bytes in a separate control. None of
these results establishes production pin provenance or general config authority.

The decisive cut control first validates the exact old missing record. It then
publishes a replacement commit immediately before fallback and removes that ref
immediately after fallback. Default policy returns status zero and the introduced
blob. Disabled policy retains the original missing result, including diagnostic
bytes/status. A subsequent read again sees the original absence. Thus a final
comparison of ref state alone cannot detect this ABA interval.

The passing test **characterizes an unsafe result**. Its retained
`fault_admission_gate=false` is not converted to true by a green libtest verdict.
This step does not implement a consistent-view or fallback binding protocol.

## First refusals and test hypotheses

The first version stops on an incorrect expectation that `core.quotePath` changes
this installed Git's Unicode tree `show` output. It does not. The corrected
control retains equal rendering as a negative hypothesis rather than fabricating
a config-drift result. The second version reaches object removal and incorrectly
expects `Process`; Git instead emits a successful batch `missing` record, so the
old object pin correctly produces `Protocol(Header)`. Both first failed outputs
and source versions remain in the packet. These expectation fixes do not change
the reader or widen a deadline.

The third version passes all 9 decoder, 1 fault and 6 transport cases on Rust
1.94. The first MSRV command compiles but the full preparation hits its original
30-second bound before fault controls, after nine decoder passes. The explicit
two-profile fault fixture then passes the new fault case on both Rust 1.94 and
1.88, as do their nine decoder cases. However, those two combined commands run
concurrently and each later fails the existing full-oracle transport case at
`Deadline`; its other five cases pass. Concurrency is execution context, not a
measured causal explanation of the timeout. These combined commands remain red;
they are neither retried nor counted as complete passes.

Final source checks pass: 31 Python cases, current all-target `cargo check`,
all-target/all-feature strict Clippy, focused rustfmt, Python compilation and
tabnanny. The previous ordinary package timeout is not rerun or completed here.

## Next boundary

Before a real caller, define and test how fallback output belongs to the requested
reference view without changing the caller's Git semantics or fresh observations.
Silently disabling replacements, expanding short IDs, refreshing pins from the
observed bytes, or only comparing context before/after the read is insufficient.
The demonstrated ABA must become a safe refusal or be excluded by a proved
supported contract. Preserve the current counterexample as its regression control.

The new fault target passes on current/MSRV, but the consistent-view admission
gate and latest combined transport regression gate remain false. Supported Linux
behavior, actual pin enrollment, whole preparation/spooling cost and the old
incomplete package gate also remain open. No performance claim follows.

All original live checks, observation frequency and 300/60/10 windows remain
unchanged. The old 780-second package guard, Cargo manifests/lockfile and
qualification manifest retain their exact bytes. No clocks, services, existing
worktrees, server, original-owner run, product workload or qualification is used.

The subsequent [pinned fallback candidate](historical-blob-pinned-fallback-result-20261024.md)
fixes this output gap with independent outcome pins and tests the same ABA cuts
as a refusal. This report and its packet remain the first pre-fix record, not
the current component verdict. Real caller admission remains closed.
