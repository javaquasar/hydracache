# Original parent loss during child construction

The local Linux x86_64 rooted-exec fixture installs a parent-death guard before
descriptor, root or credential changes. It addresses whole-process parent loss
before EXITKILL, not production custody or arbitrary multithreaded lifetime
cases. The [preregistration](diagnostic-signer-constructor-loss-plan-20261015.md)
fixes the hypothesis and oracle; the
[contract](diagnostic-signer-constructor-loss-contract.toml) records the scope.

## Guard and overlap

The existing builder captures its own TGID before its original fork. The raw
child sets PR_SET_PDEATHSIG(SIGKILL), reads that setting back and requires getppid
to equal the captured parent. If the parent already died before arming, the
signal is not retroactive: the parent check must exit 81 before further setup.
Guard failure exits 82. The guard remains through TRACEME and the first SIGSTOP.
The original owner sets its existing EXITKILL options before its first resume;
only after that resume does the child clear PDEATHSIG, before namespace or
credential changes can reset it. No child guard code allocates, locks, formats
or runs Rust destructors after fork.

Linux ties PDEATHSIG to the creating thread, whereas getppid identifies its
thread group. This slice proves actual whole-process SIGKILL only. It does not
prove a creating-thread-only death before arming while the other parent threads
live, TID reuse protection or arbitrary multithreaded signer support. The eventual
launcher must select and prove its thread-lifetime model before enrollment.
See [PDEATHSIG](https://man7.org/linux/man-pages/man2/PR_SET_PDEATHSIG.2const.html),
[parent identity](https://man7.org/linux/man-pages/man2/getpid.2.html) and
[EXITKILL](https://man7.org/linux/man-pages/man2/ptrace.2.html).

## Independent observation

Five private hooks pause the same original builder before arming, after arming,
after parent check, after TRACEME and at the first stop before SETOPTIONS. The
ordinary no-hook constructor still executes the guard. A child-only socket gate
uses prepared scalar libc and closes its inherited opposite endpoint before
waiting, so actual parent loss produces EOF rather than a retained peer. The
gate closes before namespace/exec and never becomes a helper credential.

The existing independent observer's process/pidfd, credential-checked bounded
SCM_RIGHTS parser and exact descriptor-relative nested-root checks are reused.
The constructor report has distinct magic `HC074-CTOR-v001!`, closed cut/fault
fields, original tracer TID and zero reserved fields. It is 32 bytes with five
FDs and a 128-byte ancillary buffer, after one descriptor-free progress byte.
The observer checks the original child pidfd, PPid and expected TracerPid (zero
before TRACEME), tracer TID/TGID and original directory inodes, before killing
only the original tracer pidfd.

Before arming require CLD_EXITED/81. At the four other cuts require
CLD_KILLED/SIGKILL. The isolated subreaper consumes the same original child
pidfd's exact waitid record and requires ECHILD. Only then may it chmod/remove
the four original temporary directories. No child is signaled, no replacement
is launched and no dummy is sent in loss cases. Emergency group cleanup is a
failed attempt, never a substitute proof. The ordinary test process subreaper
flag stays unchanged.

Healthy controls at all five cuts release the private gate and finish the old
original-owner ACK/close/zero-exit protocol. Closed negative controls omit only
arming (exit 83 on gate EOF), or only the original-parent check (exit 84 before
TRACEME/exec). Neither natural exit satisfies the positive oracle; they do not
claim a live orphan. Existing post-options no-EXITKILL controls remain distinct
because PDEATHSIG clears after the first resume, before the helper or dummy.

After releasing a healthy cut, seven credential-checked, descriptor-free progress
bytes identify builder/root/exec/install/READY/delivered/complete success in exact
order. At most nine packets including setup/report are accepted. Each progress
gap keeps the original five-second bound and remains a hint, not admission. The
[retained MSRV stall and preregistered amendment](diagnostic-signer-constructor-loss-progress-20261015.md)
explain the earlier aggregate healthy wait and its still-provisional pressure
attribution. Final zero-exit, original reap and empty-root checks remain exact.

## Focused gate and admission

Seven tests cover the closed role entry, five loss boundaries, five healthy
controls, two negative controls, finite terminal/frame mutations and eight
seeded fresh cases plus four independent observers. Bounds remain five seconds
per gate/progress, 30 seconds per outer process and 8192 captured bytes. No
wall-clock result is a cache performance claim.

All 123 earlier signer regressions must remain green. Helper ELF, seccomp policy,
runtime inputs, Cargo.lock, qualification manifest, production CLI and reviewed
trust inputs are unchanged. Actual private keys/tokens, issuer/job custody,
worker original-start, P2/P3 all-writer cessation, completion/cells 2–4, server
operation, qualification and ship admission remain separate.
