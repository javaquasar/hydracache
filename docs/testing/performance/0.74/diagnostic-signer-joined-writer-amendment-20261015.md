# Joined fixture image preparation writer amendment

The second final repeat failed first exec with child status 0x4800 before
installation. Do not count that attempted sequence as a successful final gate.
Retain it as signer-joined-concurrent-exec-red.log. The exact errno is still
unresolved at this preregistration; add bounded post-fork errno exits first.

Immediate descriptor closure before initial SIGSTOP does not guarantee another
in-flight concurrent fork has already reached its closure. Test the suspected
image-writer inheritance deterministically: snapshot the preparer's descriptor
table in an owned child, preserve only any current image writer and hold that
child stopped while the selected helper exec is attempted. This ninth case must
fail under parent-owned writable image preparation if that is the cause. The
snapshot child is a private finite fixture, with owned pidfd/wait/SIGKILL/drop,
no exec/adoption/secret/service/cgroup and no arbitrary command interface.

If confirmed, materialize the image through an exact /usr/bin/tee child with a
cleared environment, stdin carrying only cached fixture bytes, stdout/stderr
discarded, and an owned temporary output path. The parent holds only a pipe,
never a writable executable-file descriptor. Close input, wait for exact copy
process success, verify copied bytes and freeze mode before original launch.
The snapshot can then remain stopped while fresh exec/install/probe succeeds.
The local compiler/copy tools remain trusted fixture setup, not runtime custody.
Do not serialize or retry the measured fixture, alter seed/deadlines/policy,
admit performance claims, change normal-library code or broaden production scope.

Retain the deterministic red, then require three nine-case repeats, unchanged
standalone/regression gates and strict checks at the final source. Additional
copy/compile failures refuse; no shell or fallback copy method. GNU runtime
custody, handoff and tracer-loss remain separate. Main preregistration remains
dated October 14; the amendment and final packet record October 15.
