# Local authenticated cgroup root descriptor transfer

Baseline is `f183134cb1e37b7a94c8f60a22f6f868bb98945a`. This implements the
descriptor-transfer portion of step four of the approved
[root enrollment plan](cgroup-root-enrollment-plan-20261019.md), inheriting
R-3, R-4, R-5, R-7, R-8 and R-10. All original sources, predicates, 46 effect
brackets and absolute 300/60/10 origins remain frozen.

Use an unnamed nonblocking CLOEXEC UNIX seqpacket pair created by the actual
local owner. Capture its kernel PID/TID and real/effective credentials rather
than accepting caller-supplied credentials. Require one kernel SCM_CREDENTIALS
record matching that creator and exactly three atomic-CLOEXEC rights in
directory, kill, events order. Check actual cgroup2 identities, access modes,
root ownership, empty unfrozen domain and correspondence to names opened from
the received directory. Wrong roles, bindings, sizes, ancillary truncation or
extra rights refuse and close all installed rights. Sender and receiver consume
their single operation before attempting a syscall, including EAGAIN, failure
or missing confirmation; neither can reset or retry the attempt.

This package transfers and retains descriptors in an ordinary process. It must
not emit Armed, expose a launch/kill/completion API, or call allocator-based Rust
from a raw post-clone guardian. A later stack-safe guardian integration must
retain the original fence/timer and authenticate its reply against the actual
spawned guardian. Descriptor possession, a data ACK or same-process fixture
cannot supply that integration. Missing confirmation remains uncertain.

First retain a missing-module red control, then ordinary real-socket rejection
controls and explicit manual isolated-root role tests on GNU current and MSRV.
Check installed-FD cleanup and a real foreign sender process. Preserve prior
root SIGKILL and guardian regression controls; do not reinterpret those cuts as
new authenticated-ACK cuts. Retain source/image/recipe identities and raw-byte
receipts separately from product performance evidence.

The [Linux UNIX socket interface](https://man7.org/linux/man-pages/man7/unix.7.html)
defines per-message credentials and SCM_RIGHTS open-description transfer;
[recvmsg](https://man7.org/linux/man-pages/man2/recvmsg.2.html) provides atomic
CLOEXEC and truncation flags. These checks do not defend against a privileged
host adversary capable of impersonating credentials or changing the kernel.

No server access, installation, supervisor operation, original manager send or
product workload. Only fresh explicitly marked local WSL cgroup fixtures may
use root privileges. Independent guardians, original-frame join, held delivery,
all-writer cessation and joined SIGKILL/lost-ACK gates remain next.
