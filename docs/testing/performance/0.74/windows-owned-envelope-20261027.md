# Old-cache cleanup refusal and owned Windows envelope preparation

Preparation advances, but no full Cargo package/workspace gate is dispatched.
The cache deletion requested by the operator is **blocked by the tool before
process creation**; it is not reported as a cleanup success and is not retried
through another API. Source code, evidence and the warm main debug dependencies
remain intact. The resource prerequisite is now observed satisfied independently
of this attempted cleanup.

Tool commit: `624cf170667c5da1c6ed01d38c72de516687c089`.
The [raw packet](local-runs/windows-owned-envelope-20261027/index.json) retains
audits, the explicitly labelled tool-response transcription, first/final source
versions and test streams, native fixtures and the clean-commit preflight.

## Precisely scoped old-build targets, not a blanket target removal

All candidates are beneath `C:\Workspace\prj\jq\cashe\hydracache-074`:

| Candidate | Files | Logical bytes |
| --- | ---: | ---: |
| `target/debug/incremental` | 2,232 | 2,835,268,439 |
| `target/xtask-verify-39960/debug/deps` | 1,101 | 1,544,504,939 |
| `target/security-patch-msrv188/debug/deps` | 1,461 | 4,337,871,378 |
| Total | 4,794 | 8,717,644,756 |

These are approximately 8.12 GiB of logical compiler-cache/dependency artifacts,
not a promised physical-volume recovery. Hard links, concurrent allocation and
later rebuilding can affect physical capacity. The old isolated target metadata,
generation outputs and diagnostic contexts are not selected. In particular, do
not delete the whole 074 target, its main debug deps, diagnostic executables,
traces, receipts, bundles, first failures or source snapshots. Later MSRV work
would rebuild the selected old 1.88 dependency cache if it were removed.

The read-only audit resolves exact absolute targets, checks ancestors and all
entries for reparse points, finds zero tracked files, inventories extensions and
observes no current compiler/target consumer in its process snapshot. It does
not claim to enumerate every kernel file handle or reserve these paths against
future unrelated builds. The subsequent native PowerShell deletion command is
rejected before starting. Its recorded refusal is a **manual transcription of
the tool response**, not a fabricated OS stdout/stderr log. All three directories
are independently observed still present afterwards. No fallback, elevation,
alternate deletion API, system setting or foreign process change is attempted.

If the operator still wants this cleanup, after checking that no build has
started, the exact already-audited command is:

```powershell
Remove-Item -LiteralPath 'C:\Workspace\prj\jq\cashe\hydracache-074\target\debug\incremental','C:\Workspace\prj\jq\cashe\hydracache-074\target\xtask-verify-39960\debug\deps','C:\Workspace\prj\jq\cashe\hydracache-074\target\security-patch-msrv188\debug\deps' -Recurse -Force
```

These generated build artifacts are rebuildable, but `Remove-Item` itself does
not place them in the Recycle Bin. The command does not remove a worktree.

## Resource recovery predates this attempt

The first new volume observation already has 80,254,029,824 bytes free, rather
than the [previous report's](windows-workspace-preparation-20261027.md) roughly
5.2 GB. This is a separate tool reading, not proof of bytes freed by this task.
The first retained cache-audit sample has 80,133,902,336 bytes free. On the clean
tool commit, preparation observes **80,806,412,288 bytes** (about 75.26 GiB),
which satisfies the original 16 GiB initial prerequisite and 4 GiB running floor.

The later raw resource sample shows a 33,634,439,168-byte pagefile logical length,
CIM free physical memory of 9,582,032 KiB and pagefile usage of 55 MiB. Those
values differ substantially from the earlier high-pressure session; the task
does not assign their cause to an unobserved user/OS action or claim pagefile
logical length is allocated-volume recovery. No pagefile, clock, service, VM or
foreign process is modified. Neither free-space sampling nor restored RAM is a
hard compiler/pagefile reservation or an execution verdict.

## New private owned-Job mechanism, still no Cargo dispatcher

`tools/windows-workspace-preflight-074/owned.py` is a private CI prototype with
no CLI. It loads only the unchanged original Windows Job helper, raw SHA256
`8b392811e106eb7e692e21ee5b55cd04ea8ac421cad015edb66ce736b08c2ab1`.
Its fixed specification keeps:

- Package: the complete 156-target command, 780 seconds including compilation,
  four case threads, 4 GiB floor and the original incremental/build-jobs controls.
- Workspace: the original `cargo xtask verify`, 4800-second captured outer bound,
  16 GiB initial prerequisite, 4 GiB running floor and original incremental control.
- Five-second capacity polling and five-second owned CI retirement, with no
  retry, deadline extension, target filter or old-result concatenation.

The single-threaded worker inherits already-separated stdout/stderr. The
prototype does not replace global standard handles, descriptors or environment.
The original helper creates the child suspended with the Job in its creation
attributes, verifies membership and only then resumes it. Only the held Job and
original child handles can be retired: no PID reopen, `taskkill`, breakaway or
foreign-process control is introduced. A zero-exit leader with a live descendant
is not a completed job. Failure in setup, capacity, cancellation, retirement,
handle release or its five-second budget cannot become success.

This is CI containment, not production writer authority. It does not implement
authenticated original delivery, a monitored product worker or production
all-writer cessation. It is not yet connected to fixed Cargo execution; the
existing preflight intentionally keeps `launch_ready=false`.

## Separate actual checks and retained first errors

The final complete Python command passes **24 cases**: ten envelope models,
three tiny native Python/Job fixtures and eleven existing preflight controls.
The native fixtures establish exact separate binary stream bytes, refusal before
spawn on the synthetic capacity floor, and a zero-exit original leader with a
confirmed live descendant timing out rather than passing. Each fixture retains
its own original stdout/stderr/result. Small synthetic fixture bounds are not
overrides for the package/workspace budgets.

The first new command remains red: 20 passes and two model-clock errors in 22
cases. Both mocks exhaust a finite timestamp list during final cleanup. The
corrected mocks use a stable explicit clock state; neither the runtime mechanism
nor a live deadline/check is changed. The intermediate complete command passes
22 cases; two subsequently added cancellation/cleanup-deadline controls yield
the final complete 24-case result. These are distinct commands, not joined
passing prefixes. Initial source bytes and all three native fixture generations
are retained.

Separately, all **six** unchanged native Windows CI Job controls pass: owner loss
before Resume, owner loss with a live writer, post-create setup refusal,
unsupported Job-list refusal, a zero-exit root leaving a writer, and Unicode
argv/CWD with inherited streams. They target only their own synthetic processes.
No product, original-owner, WSL, service or server test is started.

All six Python modules parse. Fresh preflight on the **clean tool commit** passes
the existing 394-Rust-body/58-auxiliary/26,832-site/156-target source closure,
original Windows environment guards and all 36 workspace member manifests.
It records clean source status and the now-satisfied resource prerequisite, but
does not turn any package/workspace/admission flag green. Source review is not
compiler expansion or a whole-workspace effects proof. Rust source, Cargo.lock,
MSRV, qualification identity, live predicate frequency/order and 300/60/10
origins/windows remain exact.

## Remaining boundary

Resource headroom is no longer the observed immediate blocker; successful cache
deletion is still unperformed. Next connect and independently review the **fixed**
Windows worker/dispatcher with fresh source/ref/configuration/environment custody,
exact owned capture and complete target accounting. Only then run one distinct
full package command under 780 seconds and the separate complete workspace
milestone under its original envelope. Do not replay V9 unchanged, combine
suffixes, weaken security or declare the whole release admitted from these
synthetic controls. Installation, server tests and product load remain out of scope.
