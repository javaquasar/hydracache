# Bootstrap document reader compatibility

The narrow bootstrap source-test boundary now passes 53 unchanged bootstrap
cases, ten preceding proof cases and six new controls on Rust 1.94 and MSRV 1.88.
This closes fifteen bootstrap failures, not the full package gate. The separate
complete current-tree audit still refuses 26 inputs; it is not excluded, pinned
from observations or relabeled green.

Implementation commit: `03af9ad2916890ac585bbf304084c1391d60eede`.

## Baseline and scope

The baseline on `181c077abdfbae5a8c4dfe67cee6f1c95519d529` runs nine bootstrap
targets and the complete audit: 54 cases, 38 passes and 16 failures. Fifteen
failures belong to bootstrap; one audit failure enumerates the 26-input frontier.
The retained logs identify changed document phases, not broken production
bootstrap semantics. The bootstrap guards already understand their own earlier
source additions. Projecting every input back to an older pre-bootstrap tree
would erase required hooks and is deliberately not done.

Only `docs/COMPAT.md` and `docs/testing/gated-test-registry.toml` receive the
existing independently pinned successor projection before their original guard
chain. Their current producers are `a67a5299ab335424e9b11330dc06bd749ea9b859`
and `958b3135a15c00b9f6aad41522cef9d1f6608d3a`; the exact predecessor for both
is `3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f`. Whole current/predecessor hashes,
ordered inverse edits and the unchanged original body assertions all apply.

There are seventeen replaced read sites in nine source-test callers. Every call
still observes once, in the same order, and propagates the original I/O error.
Unknown paths retain their raw bytes. Only immutable expected descriptions are
cached; no successful observation becomes permission for a later observation.
Lexical path selection is not symlink/inode authorization. Actual callers use
fixed repository inputs and are checked in full.

## Keep the earlier proofs intact

Two further test-reader edits preserve the existing complete-source proofs:
the owned-case installed-reader proof constructs the exact extended caller;
the authority-phase proof verifies that extended owned-case reader before
exposing its predecessor. No old case, assertion, mutation loop or predicate
is removed. A separate fixed-commit generator and whole-body contract describe
all eleven caller edits; AST comparisons retain all 63 preceding cases and
every assertion token sequence.

The final caller descriptor SHA256 is
`f780c166ff6e708eb2b09013075e414426bad8ae443f537a5ad0a08dcebfb3ce`.
The [adapter README](../../../../tools/bootstrap-doc-reader-074/README.md)
explains the fixed construction and refusal behavior. Stale, partial, duplicate
and unrelated document changes refuse; unknown source paths are not normalized.
Separate controls cover one ordered observation, I/O failures after success,
wrong roots, narrow path selection and mutation of the extended proof reader.

## First outcomes stay visible

The first candidate passes all 69 selected cases on both compilers but strict
Clippy refuses its unnecessary duplicate module imports and unused helper graph.
The first correction exposes a compile failure: most old bootstrap targets have
no root `runtime` module. That result is retained too. The final correction gives
standalone readers their existing projection module and the installed reader
its already-imported projector, sharing the one-read/path-selection core. It
does not add a lint exception or change document validation semantics.

The [raw packet](local-runs/bootstrap-doc-reader-20261026/README.md) retains
every invocation, raw stdout/stderr, source/HEAD receipts, both failed outcomes,
fixed original caller bodies, producer documents, exact deltas and final source.
Earlier descriptor drafts are retained as drafts, not as additional test runs.
Offline replay does not execute captured source. All 69 final cases pass on
each compiler; scoped formatting, all-target check, strict all-target/all-feature
Clippy, doc-check and the local performance-contract check are separate checks.

## Remaining gate

The audit frontier comprises fourteen newer supervisor modules and twelve
terminal GET-owner/control/proposal inputs. Those need their own exact current
source accounting; this adapter cannot approve them. The first full package
failure and the old source/effect review's stale-source refusal remain intact.
A new complete source/effect review is needed before another package invocation;
passing cohorts cannot be spliced into a package success.

No runtime/dependency change, live-check frequency/order change, deadline or
threshold adjustment, clock/service/server action, installation or qualification
occurs. GET-owner stays terminally rejected, C74 unresolved and release admission
closed. This result is source-test compatibility, not a cache performance claim.
