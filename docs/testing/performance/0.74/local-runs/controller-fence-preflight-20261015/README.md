# Controller verification under the original common fence

This local P2 preflight links original controller/OIDC/kernel/authority checks
to the existing common empty fence. It publishes nothing, consumes no nonce,
and supplies no authenticated original-start or server admission. See the
[preregistration](../../diagnostic-controller-fence-preflight-plan-20261015.md),
[design](../../diagnostic-controller-fence-preflight-design.md) and
[contract](../../diagnostic-controller-fence-preflight-contract.toml).

## Baseline and test first

The unchanged-source baseline at `5ae10682` passes 17 OIDC and 11 empty-fence
tests. The new positive composition test fails with E0599 because
bind_common_empty_fence does not exist. The snapshot includes the exact test
and its registration; the no-index Git diff exits1 because files differ, not
because capturing the snapshot failed.

The opt-in implementation borrows the original OIDC chain and owns the already
held original fence. Its five checks are fence/controller/fence/controller/fence.
Every first error stops observations and cross-refuses both original chains.
Constructor failure retains the refused fence; healthy destruction releases
only the original lock. No new journal, root, verifier or replacement input is
selected, and no existing verifier, fence or durable attempt behavior changes.

## Local checks and negative diagnostics

Before failure-value boxing, eight focused tests pass in three independent
default-parallel runs. They cover
eight late contexts, ten original input/kernel/policy faults, four original
lock/directory replacement cases, healthy and refused lock lifetimes, all five
bracket positions and four independent concurrent chains. Seed: `0x0740c093`.
The broader original trust/controller/kernel regression passes 57 and the
unchanged empty-fence regression passes 11. Linux Rust1.88 passes 25 OIDC tests,
including the new eight. These are synthetic issuer and filesystem models with
actual local kernel packet/context observations, not actual production trust.

The first compile-fail run returns cargo exit0 and reports 9 passed, but Rust1.94
internally panics while diagnosing a missing extraction method. That capture is
not accepted as API-proof evidence. The example instead tries the private
original field and receives E0616; both Rust1.94 and Rust1.88 then produce nine
ordinary expected compile errors without an internal panic. No constraint,
runtime code, compiler version or threshold is relaxed to accept the old run.

The first Linux MSRV strict lint rejects the constructor's at-least176-byte
Err variant. The correction keeps the original refused fence in a Box owned by
the failure value; it changes neither successful ownership nor lock lifetime.
The rejection is retained separately, with no lint suppression. A compiler type
size diagnostic is not an allocations/op or product performance measurement.

After boxing, the final-source trust regression passes 57 and Linux MSRV's OIDC
group passes 25 again. Both affected packages pass all-target/all-feature checks
and strict Clippy on Windows/Linux Rust1.88/1.94. The static suite passes 166
tests; six registry tests pass and all 176 gates remain registered unchanged.

The first Windows static-test link ran out of disk space before any test executed.
Eight obsolete ignored debug artifacts from this worktree were permanently
removed after checking exact paths, age and absence of compiler/test processes;
the cleanup recovers 1918267392 bytes of measured free space. They are rebuildable
artifacts, not source/evidence or another worktree. The identical command then
links and passes 166. The initial PowerShell source-audit helper also refused
because tuple/string construction was incorrect; explicit named records fix the
helper, and exact old-source/contract comparisons pass. Both negatives are retained.

Final source/check/repetition identities and all command statuses are recorded
in [manifest.json](manifest.json). The verifier checks retained artifact/source
consistency, not signed execution attestation. Windows build/lint checks do not
execute Linux-only fixtures. No full release qualification or performance claim
is inferred from the package gates.

## Remaining execution rights

Runtime refusal is not a persisted authority revocation. An empty fenced
preflight is not a retained attempt, globally consumed nonce or atomic snapshot.
Existing early-attempt, publication and start APIs do not consume this result.
A writable join must carry the same original fence and authority into durable
request/nonce acknowledgement without reacquisition or recovered permission,
then pass its own SIGKILL/lost-ack matrix. Real receiver/runtime/job policy pins,
authenticated original-start, watchdog, whole-writer cessation, completion and
cells2-4 remain open. No private key/token, product/server workload, workflow,
push, frozen qualification input or Release0.73 artifact is changed.
