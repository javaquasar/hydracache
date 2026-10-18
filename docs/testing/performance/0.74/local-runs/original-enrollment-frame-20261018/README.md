# Combined original custody and barred enrollment validation

This is a separate nonexecuting experiment under the
[approved frame plan](../../diagnostic-original-enrollment-frame-plan-20261018.md).
Its base is 27dbbac7, immediately after preregistration on 3a12b722.
The [previous stopped matrix](../original-collector-loss-20261018/README.md)
and its MSRV Challenge(Time) refusal/101-0-0-0 ABBA attribution remain unchanged.
New results cannot retroactively accept that withdrawn candidate.

## The changed boundary

The old sequential path constructs custodied preparation with 17 opening and
17 closing authentication/signature calls, then adds a full 34-call enrollment
frame. The new private owner puts both barred child enrollments between the
first opening and closing observations. Healthy construction must assert 34
calls, and later revalidation must independently assert another 34. This is
per kind: 34 authentications and 34 signatures, not a combined total. It is
the approved removal of an intermediate observation, not reuse of a cached live
observation. It is not a product throughput, allocation or CPU claim.

OriginalEnrollmentFrame owns, rather than borrows from itself, the custodied
original preparation. Its collector and custodian fields drop before original
messages, pipes, sinks and the last custody alias. Failure owns the whole refused
object. Cancellation attempts use the unchanged exact pidfd stop/reap paths;
destruction is not authenticated active cleanup or completion evidence.

Old constructors/validators, original material and time predicates, both raw
child implementations and the generic O_WRONLY watchdog remain exact. The new
entry uses original O_RDWR sinks and the same ten-FD inventory, compares that
inventory after closing, observes both children around the closing round and
checks original custody/alarm/challenge age before marking readiness. Reserved
300 s, Starting 60 s, progress 10 s and challenge age 10 s keep their original
origins. Neither P nor B can activate a worker or authorize send/completion.

## Reproduction and fault interpretation

Both release-libtest images are built locally under WSL Ubuntu with explicit
Rust 1.94.0/MSRV 1.88.0, --locked, no-run, and separate target directories.
`build-bindings.json` records 326 inputs: all top-level package Rust sources,
root/package Cargo manifests and Cargo.lock. It does not claim to inventory
registry dependency sources, bin targets, toolchain or host environment; the
lockfile, compiler versions and exact tested image hashes are retained separately.
Fourteen full source capsules on the exact base reconstruct the candidate,
including private fixture protocol and manual gate registrations. The verifier
also requires the accepted source's complete tree diff to equal those fourteen
paths, so embedded assets and other tracked build configuration inherit the
unchanged base. This is not a bit-reproducible toolchain/environment claim.
The retained executed Python scripts are byte-exact. Before committing source,
only an extra blank EOF line in each of the two new Python files was removed;
the verifier requires script equality apart from trailing newline count. Fresh
fast parser units also run against the committed formatting. No Rust/image,
runtime statement, protocol or result changes with this formatting correction.

`run.sh` executes only those libtest images and Python fault tooling as nonroot
UID/GID 1000 in /tmp/hc074-enrollment-frame-20261018.jj6xm5. It reads the
pre-existing signed data bundle but executes none of its product programs.
Read-only guards bind boot, service identity/restarts, installed image/receipt
hashes and absence of campaign/lifecycle/authority markers. No installation,
supervisor operation, clock mutation, product workload or qualification runs.

The fixed order is both healthy screens first, then the complete 1.94 cohort
and complete 1.88 cohort. Each has three healthy controls, nine reason-specific
refusals, three actual lost-B controls and twelve owner cases. Seeds 0x0740e118,
0x0740e318 and 0x0740e418 identify fixed enumerations, not random coverage.
The script stops at the first unexpected result; no automatic retry is allowed.

Lost B consumes the real first notice and requires first remaining E, not merely
an Expired error. Owner cases cover three repetitions of both closing-round
cuts and both child-death orders. The observer receives only two atomically
created pidfds over exact kernel-credential-checked SCM_RIGHTS. Its distinct
hc074-enrollment-owner-test-v1 protocol uses boolean frame_closed, not an
invented authentication count. Healthy controls independently assert counts.

The external driver kills its exact unreaped owner, observes both adopted
children and autonomous closure of four collector I/O FDs, then kills/reaps
either child first. The survivor must retain the fence past the unchanged
absolute bound. After the last exact child dies, kernel lock availability must
still fail to admit retained history. Records/authority snapshots stay unchanged.
Its five-second post-failure FD observation and cleanup fail-safes cannot renew
an admission window. Selected signals exclude ordinary cancellation and cleanup.

## Results

Source 08c4802f544a9787798b3e5db328e175d3a00435 completes the selected
two-compiler gate without rerunning the cohort: eight healthy controls, eighteen
reason-specific refusals, six actual lost-B controls and twenty-four owner
SIGKILL cases. The latter select seventy-two signals, excluding cancellation
and cleanup. Each compiler completes all cuts/orders and three repetitions.
Both fresh screens precede either fault cohort. All original bounds remain.

Each image passes eight descriptor/notice units and thirty-two storage units;
the host passes fifteen Python units. Local checks pass 225 static contracts,
two private compile-fail doctests, 44 portable Windows library tests, affected
Windows/current GNU/MSRV all-target checks and strict all-feature Clippy,
formatting, registry and the local non-promotable performance contract. After
the EOF-only Python formatting correction, nine parser units and the four
new source contracts pass again. Raw logs distinguish those separate runs.

Postflight finds no fixture executable and preserves supervisor PID 8839,
zero restarts, boot and installed image/receipt identities. The closed manifest
and read-only verifier bind source, scripts, images and every selected row.
Run `pwsh -File verify.ps1 -SelfTest` from this directory for positive and
negative receipt checks; this does not rerun host processes or grant admission.
Full workspace milestone verification is not rerun for this isolated slice.

## Remaining admission boundaries

These are synthetic barred-child correctness proofs, not every instruction
boundary or an active execution fault matrix. Original authenticated send/start,
whole-writer-tree enrollment and cessation, external active supervision,
explicit active-cleanup failure handling and durable completion remain separate.
No release installation, pilot, expensive qualification or ship is admitted.
The frozen qualification-manifest SHA256 remains
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
