# Retain the original exec environment descriptor before hardening

Amends `7cdf6c8461533e0a644c02cb1253ee15d5635ed6` before final candidate
checks. The first nine-case run passes policy/fault/SIGKILL tests but five tests
refuse EACCES after ND0. The owned access diagnostic isolates the refusal to
opening environ: namespace/root/cwd/exe, UID/GID maps, status and maps remain
readable. Keep both failed run and diagnostic; neither proves post-install denial.

Keep the helper, 64/512 policy, original hardening order, all context checks and
bounds. Do not set dumpable back to one, skip empty-env checking, use privilege,
change root/namespace policy or fall back on reopen failure. Instead the private
namespace selector's owner opens its own original proc environ file at the exact
first exec stop, before startup and ND0. Retain a readonly CLOEXEC/NOFOLLOW procfs
descriptor and its original device/inode identity; export it nowhere. Only the
namespace profile uses it, and old rooted tests keep their original behavior.

For every later context check require that original identity, regular/procfs and
readonly/CLOEXEC flags, then a one-byte positional read at zero returning EOF.
No arbitrary address, nonempty environment bytes, PID, path or caller FD is read.
The original thread-bound trace/pidfd chain and exec denial are necessary: a proc
environ file retains an mm reference and cannot independently prove current exec
after re-exec. The [Linux6.6 proc implementation](https://raw.githubusercontent.com/torvalds/linux/v6.6/fs/proc/base.c)
checks access at open and stores the mm used by environ_read. This informs the
mechanical hypothesis; local tests must confirm actual retained reads after ND0.

Add retained-vs-reopened ND0 comparison, missing/wrong regular/proc FD, wrong
identity and descriptor-flag refusal tests. Continue live post-ND0 checks of all
other original context fields and post-probe mappings. The old-policy nested-
namespace control must still observe a changed namespace and regained effective
capabilities, then refuse/reap. No stale descriptor may authenticate a replacement,
and no handoff, signing, production custody or server admission is enabled.
