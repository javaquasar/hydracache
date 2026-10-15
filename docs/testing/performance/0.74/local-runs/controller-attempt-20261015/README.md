# Original controller request attempt local evidence

The [design](../../diagnostic-controller-attempt-design.md) retains one original
Reserve request under the same controller/OIDC/kernel/authority chain and common
fence. It does not grant start or lifecycle rights. All positives use synthetic
issuer/controller/OIDC fixtures and private nonroot roots with actual local
kernel IPC/context observations, not production enrollment.

## Baseline and development diagnostics

The unchanged baseline at `9d754825` passes eight original common-fence preflight
tests. The test-first new positive fails with E0599 for the absent retain_request
method. The retained test-first probe patch reconstructs the two test additions
on that baseline. Its diagnostic is developmental evidence, not a final-source run.
The reconstructed patch's first hunk lacked trailing context and fails the
baseline-index check; the corrected patch passes in an isolated temporary index
without changing the worktree or real Git index. Both patch versions and command
results remain retained; this is a patch-packaging correction, not a test change.

The first implemented positive passes. A later eight-case run has seven passes
and one fixture failure: after dropping the retained attempt, the test acquired
a campaign claim to demonstrate lock release. That acquisition also created a
campaign marker, so the subsequent fresh empty-fence check correctly rejected
campaign context instead of returning the expected fixed-leaf Conflict. The
test now uses the existing lock-only operation. No rejection predicate or
retention behavior is weakened. A subsequent nine-case run passes; the final
suite adds a separate exact-capture mismatch test.

The first current-Linux all-target check succeeds, but the following strict
Clippy reports corrupt incremental artifacts, invalid rmeta and a compiler
panic. That failed build is not accepted. The named loadgen rmeta is 12977255
bytes with two hardlinks; an independent read finds its first 32 bytes zero.
The origin of that corruption is not established by this inspection.

After resolving the exact WSL cache path and confirming no active compiler,
Cargo clean removes only cached artifacts for xtask, hydracache-loadgen and
hydracache-long-run-supervisor-074 from
`/home/artur/.cache/hydracache-074-194-check`. Cargo reports 19616 files/12.1GiB
logical removal; that is not a measured Windows free-space delta. These are
rebuildable files, not source, retained evidence, another worktree or MSRV cache.

## Scope of the acknowledgement

Creation is exclusive and descriptor-relative, with the existing eight bounded
write/seal/sync cuts and final original binding checks. Failure retains context
and the common lock until drop. Refusal is permanent at runtime; it is not a
durable authority-refusal append. Legacy attempt publication explicitly rejects
the controller filename. The new canonical reader returns only byte consistency,
not a runtime acknowledgement or signature attestation.

Any retained leaf context blocks a fresh empty-fence reservation. BeforeCreate
is unadmitted; readable complete bytes cannot recover permission after lost
acknowledgement. No global nonce ledger or whole-host rollback proof is claimed.
The joined original-kernel SIGKILL/lost-ack matrix is still required. Logical
callbacks and previous storage-only process-loss results do not close it.

The manifest binds command captures to historical source objects and frozen
inputs. Its verifier establishes unsigned consistency, not trusted execution
attestation. No full release verify, server/host operation, workflow, real key or
token, product workload, qualification, push or performance claim is included.

## Final local gates

Runtime implementation is `865dab48b7224d65b7586f5cdcaf0534a729bfb3`.
Final source including the separate static gate is
`f459aff18504d5f17ecdd908c0a1af5bb326aa35`; preregistration is `bc9f0f7e`.
The package started on October 15 and its final checks completed October 16
(Europe/Malta). Documentation/evidence remained uncommitted during the checks;
the packet is not a clean-tree build or execution attestation.

The final-source trusted-input suite passes 67 tests. All ten new cases pass
in three independently started four-thread runs and once on Rust1.88. Each
compiler's seven API examples emits seven ordinary expected errors, including
E0616/E0382, with no unresolved import or compiler panic. The legacy intent
group passes 124 with nine manual cases ignored; those nine process-loss cases
then pass separately. All eleven empty-fence cases pass.

Both affected packages pass all-target/all-feature check and strict Clippy on
Linux/Windows Rust1.88/1.94 after the scoped cache repair. Windows checks do not
execute Linux-only fixtures. The static suite passes 167, registry tests pass
six and all 176 gates remain registered unchanged. Formatting, doc-check and
the local non-promotable performance contract pass; require-ship returns its
expected closed-admission failure. Full release verification is not claimed.

[manifest.json](manifest.json) inventories normalized UTF-8/LF command output,
statuses, source blobs and frozen input hashes. Run [verify.ps1](verify.ps1)
to check consistency without rerunning tests or cache cleanup. The new joined
SIGKILL/lost-ack matrix remains open despite the successful legacy regression.
