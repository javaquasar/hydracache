# Original tracer loss with an independent local observer

The private Linux x86_64 fixture tests actual SIGKILL of the original tracer at
six original-child boundaries. The separate monitor retains the original child
pidfd and requires CLD_KILLED/SIGKILL, exact reap and ECHILD, without signaling the
child on the successful path. This proves the fixture's EXITKILL behavior, not
production launcher custody or worker authorization. The
[contract](diagnostic-signer-tracer-loss-contract.toml) keeps those admissions closed.

## Original ownership and bounded report

The ordinary test launches only the current libtest executable's exact closed
entry, with cleared environment and synthetic role/stage selectors. A fresh
process group contains its isolated monitor, tracer and child. Only that monitor
sets the subreaper flag; ordinary test-process flag reads bracket every scenario.
The tracer reuses the unchanged original rooted-exec/handoff owner and GNU helper.
Original construction, root, exec, exact filter, ND0 and protocol checks are not
reimplemented or replaced by an adopted PID.

One unnamed private report carries 32 bytes and exactly five SCM_RIGHTS FDs: the
original child pidfd and four original root directory descriptors. The 128-byte
aligned ancillary buffer receives kernel SCM_CREDENTIALS with SO_PASSCRED;
original tracer PID/UID/GID must match. A per-thread TID is checked against actual
TracerPid and its original tracer TGID. Pidfd type/fdinfo and child PPid bind the
target before fault injection. The production ancillary receiver still refuses
RIGHTS; only this private observer protocol admits this exact fixed descriptor set.
All received FDs are owned before payload/credential refusal, and an unvalidated
received pidfd is never a signal target. The inherited report endpoint becomes
CLOEXEC before setup utilities execute; the original dummy channel is separate.

Before the report, exactly stage+2 credential-checked one-byte progress packets
advance0..6 without descriptors. At most eight packets including the final report
are accepted, with the same five-second gap and30-second outer bound. These are
setup hints, never authority or readiness. Wrong/duplicate progress refuses. The
[progress amendment](diagnostic-signer-tracer-loss-progress-amendment-20261015.md)
retains pressure failures caused by charging compile/root/all stages to one
aggregate report timer; no timeout was increased to hide those failures.

The observer's original retained temporary parent is explicitly0700. The unchanged
runtime materializer nests `.tmpName/root` below it; its container is0755 and
four readonly runtime directories0555. Descriptor-relative DIRECTORY/NOFOLLOW
opens and original inode checks reject a different root, shape or symlink. An
earlier direct-child-root assumption and an incorrect default0700 assumption both
refused before fault admission. They remain in the
[path amendment](diagnostic-signer-tracer-loss-root-amendment-20261015.md) and
[mode correction](diagnostic-signer-tracer-loss-mode-amendment-20261015.md).

## Terminal proof and controls

After the report, the child remains at its original trace stop. The positive
path signals only the original tracer pidfd, observes that tracer's SIGKILL,
then requires the independently retained child pidfd's SIGKILL terminal record.
The isolated subreaper consumes that same record and requires ECHILD on repeat.
Only then are the four original temporary directory FDs chmodded for cleanup.
The [ptrace reference](https://man7.org/linux/man-pages/man2/ptrace.2.html) defines
EXITKILL; [subreaper behavior](https://man7.org/linux/man-pages/man2/PR_SET_CHILD_SUBREAPER.2const.html)
and [pidfd waitid](https://man7.org/linux/man-pages/man2/waitid.2.html) provide the
independent reap path. A five-second logical stall or 30-second outer stall refuses,
with 8192 bytes of nonsecret captured output. Emergency owned-group/tracer cleanup
is failed evidence and never satisfies the positive death check.

The first control prediction without EXITKILL fails: installed yields natural
exit74 rather than73. Tracer death and endpoint teardown are not an atomic
boundary for the untraced child. The
[retained negative result](diagnostic-signer-tracer-loss-control-result-20261015.md)
preregisters a separate source-derived finite oracle: installed73/74, ready74,
delivered0/76, always CLD_EXITED and never SIGKILL. Actual tuples are logged; the
positive SIGKILL predicate is unchanged. These controls demonstrate a different
death cause, not a reproduced live orphan. Healthy delivered control completes
the unchanged ACK/close/zero-exit protocol under its original owner. Six report
faults require their intended parsing/frame/kernel/directory refusal, not any
unrelated error that happens to fail.

Eleven tests include the closed role entry, six-stage actual loss, controls, healthy
completion, report faults, pipe-EOF proof of received writer closure, exhaustive
finite death oracle,256 frame mutations, eight fresh cases and four independent
monitors. The pipe EOF
oracle runs after exec in a separate closed role with no subsequent fork;
[its amendment](diagnostic-signer-tracer-loss-fd-amendment-20261015.md) retains the
earlier false attribution caused by unrelated concurrent fork inheritance.
No wait-any, PTRACE_ATTACH,
caller command/PID/FD API or old-guard conversion is introduced. The constructor
window before EXITKILL options is outside this positive proof. Failed feasibility
cleanup can leave owned temporary forensic files; it is not positive cleanup or
production survival evidence. Independent runtime/job/issuer custody, real token
producer/receiver/replay/fence, worker P2/P3 and all writers, completion/cells2-4,
server installation/rehearsal and qualification remain separate.
