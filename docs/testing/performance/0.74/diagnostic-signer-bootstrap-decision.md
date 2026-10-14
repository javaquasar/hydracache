# Signer bootstrap launch and runtime decision

Status: operator approved the local-only prototype on 2026-10-14. Actual runtime
custody, production inputs and external operations remain separate. Audited source is
`7e4d0e3ef78cdcf3455b8719968e8e888abeeda8`. Existing executable observation
does not establish original invocation. The next useful step must control the
original launch and the runtime before a credential handoff, rather than add
another asserted observation and call it authorization.

## Current boundary

The [executable packet](local-runs/diagnostic-signer-executable-20261014/README.md)
contains an actual same-inode re-exec that still passes generation and executable
checks. A fresh eight-case run at the audited source reproduces that result in
the [decision audit packet](local-runs/diagnostic-signer-bootstrap-decision-20261014/README.md).

`AssertedSignerBuildSelection` authenticates statements under an asserted key;
runtime and bootstrap files are opaque. `AssertedSignerBundle` checks retained
files, not trusted ancestry. `AssertedSignerExecutableRead` compares kernel
objects, not invocation history or executable memory. No production producer
consumes these guards. The existing producer can sign a retained request without
reopening its public inputs, but does not require bootstrap or verified OIDC
coordination. Its CLI still has separate audience/sign invocations.

Production `pin_kernel_process` admits the fixed diagnostic worker scope. A
signing job is a different principal and lifetime; widening that scope or
exporting the test-only PID capture would weaken the existing boundary. Signer
launch needs its own private original-owner chain and reviewed purpose.

## Recommended runtime choice

Preserve the selected Rust 1.94 release/System profile and
`x86_64-unknown-linux-gnu` target. Add a separately reviewed trusted launcher and
an independently selected read-only runtime root. The actual ELF determines
whether an interpreter and shared libraries are needed; the GNU target alone
does not prove a particular linkage. Inspect the admitted artifact, then account
for its interpreter, permitted libraries and later loads, loader configuration,
environment, TLS trust inputs and inherited descriptors. A hash of an opaque
manifest or a scan of current mappings cannot replace that selection and custody.

This is preferable to silently changing to static/musl: that alternative changes
the fixed build contract and needs separate dependency, TLS, supported-target
and compatibility checks. Static linkage may reduce runtime inputs but does not
prove original exec, prevent re-exec or authenticate issuer/job authority.

The proposed trust base explicitly includes the admitted kernel, runner
administration, independently authenticated launcher/runtime and reviewed signer
code. No untrusted same-UID process may retain mutation/debug access to the
signer or its inputs. Kernel/root compromise and arbitrary exploitation of the
reviewed signer are not defeated by file hashes. If protection against those
actors is required, this proposal is insufficient and needs a different
isolation or attestation design. It does not claim mapped-memory attestation.

## Local prototype after approval

Preregister exact files, syscall/ABI policy, owned-helper protocol and failure
cleanup before implementation. Use only synthetic material and owned nonroot
helpers. Do not mount an image, alter accounts/services/cgroups, read a real key,
retrieve OIDC tokens or dispatch a job. The prototype evaluates launch mechanics;
its temporary root remains a fixture, not production custody.

1. Validate a fixed runtime fixture and retain its original inputs. Reject
   scripts, unsupported ELF/runtime shapes, loaders/configuration outside the
   selected root, writable inputs and unexpected environment/descriptor inheritance.
2. Create the child and private handoff channels inside the launcher; never adopt
   a supplied PID. Evaluate tracing the first exec with `PTRACE_O_TRACEEXEC`,
   retain the actual child/pidfd and correlate the kernel event with the selected
   original executable. No arbitrary command/path or public launch API.
3. Before any dummy credential handoff, require the reviewed helper's fixed
   readiness protocol and install a fail-closed post-launch policy that denies
   both `execve` and `execveat`, including same-inode exec. Account for every
   thread, alternate syscall ABI and the initial permitted exec. An exec event
   alone cannot authenticate a child-supplied readiness assertion.
4. Evaluate a bounded one-shot handoff to that original child. Wrong/duplicate
   readiness, exit, event loss, channel loss or filter failure refuse. Launcher
   loss must not leave an authority-bearing orphan. Retained observation guards
   must have no conversion into the new launch owner.

Seccomp is a syscall restriction, not a complete sandbox; filters must check
architecture as well as syscall number. `no_new_privs` prevents certain exec
privilege gains but does not prohibit exec. These limits come from the
[kernel seccomp documentation](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html)
and [no new privileges documentation](https://www.kernel.org/doc/html/latest/userspace-api/no_new_privs.html).
Tracing places the launcher in the trust base; any later real-secret integration
needs a separate custody review. The prototype must fail explicitly if the
required kernel mechanisms are unavailable, not fall back to PID polling.

## Required falsification matrix

| Boundary | Local proof required before integration |
| --- | --- |
| First exec | Exactly the owned child and selected executable; foreign PID/object, exec failure and unexpected extra exec refuse. |
| Post-launch exec | Same-inode and different-inode `execve`, descriptor-based `execveat`, alternate ABI and thread paths cannot bypass policy. |
| Runtime | Unsupported interpreter, foreign/equal-byte replacement dependency, preload/configuration injection and unexpected executable mapping refuse under the chosen supported profile. |
| Handoff | Dummy data is withheld before readiness; wrong peer, duplicated handshake, descriptor leak, oversized/truncated message and loss of acknowledgement cannot grant a replacement. |
| Owner loss | Child/launcher exit and owned SIGKILL at handoff boundaries retain failure and leave no authority-bearing orphan; no automatic retry or PID adoption. |
| Composition | Old asserted model/bundle/executable guards cannot manufacture the new owner; no signing/start/cleanup consumer is enabled by fixture success. |

Unsupported runtime shapes may refuse instead of growing the supported profile.
Limits and logical synchronization signals must be frozen before tests; no
wall-clock performance threshold or product workload belongs to this matrix.
Run repeated Linux fixtures, seeded transition cases, independent concurrency,
descriptor ownership checks, compile-fail constraints, focused regressions,
Rust 1.94 strict lint and MSRV 1.88 checks. Preserve failed feasibility outcomes.

## GitHub and host integration remain separate

GitHub grants `id-token: write` at workflow/job scope, not as a late individual
step permission. Its request bearer is exposed through runner environment
variables; clearing one process environment cannot remove authority from the
whole job. See the [official OIDC permission and request reference](https://docs.github.com/en/actions/reference/security/oidc#workflow-permissions-for-the-requesting-the-oidc-token).
Build and dependency installation must stay in a distinct job without token or
signing authority. The signing job's reviewed bootstrap/runtime and all steps
need custody review before that job starts. A local handshake is not a mechanism
for retroactively making an untrusted token-bearing job safe.

After the local mechanics pass, independently select signer-build issuer/pins,
runtime/root custody, workflow SHA/subject/protection and issuer snapshot
membership/expiry. Do not repurpose the frozen observer builder or worker issuer.
Then integrate the retained request and actual bounded token producer, followed
by the authenticated receiver and durable replay/common fence.

Signer bootstrap does not implement P2/P3 worker original-start or all-writer
revocation. Those still precede completion publication, cells two through four
and their SIGKILL/lost-ack matrix. Installation and nonproduct server rehearsal
need their own admission; product pilots and six-hour/24-hour qualification
remain separately authorized. No release status, threshold or frozen artifact
changes follow from approval of this local prototype.

## Approved local boundary

The operator approved a local-only trusted-launcher prototype with the existing GNU profile,
explicit runtime selection and first-exec/post-launch handoff checks above.
Approval does not select actual production trust inputs or authorize any
credential, GitHub or server operation. Reject or revise the trust model before
integration if kernel/root trust or isolation from untrusted same-UID processes cannot
be provided at the eventual signing job boundary.
