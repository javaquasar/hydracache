# Attribute the sealed memfd mount refusal without retrying it

The first memfd open_tree precondition returned EINVAL on WSL kernel
6.6.87.2-microsoft-standard-WSL2. No detached mount or publication succeeded.
That result stays red; do not repeat the memfd probe or run the original-owner
fault matrix. The uncompleted runtime candidate is removed from compilation
and retained as rejected source text. All preceding runtime files stay exact.

One distinct, local regular-file control is permitted to attribute syscall and
namespace capability. It runs in another fresh private mount namespace and a
unique temporary directory. A synthetic child clones an ordinary namespace-backed
file, applies readonly/nosuid/nodev, publishes it and closes its source/tree FDs.
The parent keeps the namespace, SIGKILLs and reaps that exact child, then checks
same inode/content and readonly flags. Cleanup targets only its own leaf mount
after reaping; no clocks/services/server/product. The normal source is deliberately
not sealed and is not an eligible production launcher. A positive result cannot
promote the failed sealed-source candidate or satisfy its retirement gate.

The exact 6.6.87.2 kernel source open_detached_copy calls __do_loopback, which
rejects a source outside the caller mount namespace except for namespace dentries.
The first failure is consistent with the internal shmem mount backing memfd.
The control distinguishes that source-class restriction from a generally disabled
mount API. A separately enrolled bootstrap/other source representation needs a
new explicit architecture decision, not a silent fallback or copy.
