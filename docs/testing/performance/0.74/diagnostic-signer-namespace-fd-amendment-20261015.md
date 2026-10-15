# Retain the original descriptor inventory directory before hardening

The environ-only amendment's retained EOF read succeeds after ND0, but the full
eleven-case run still refuses. The expanded diagnostic shows task enumeration
available and fd-directory reopening denied. The earlier diagnostic did not
enumerate directories; its claim that all other context checks remained readable
was too broad. Retain that failed run and the expanded diagnostic unchanged.

Keep ND0 before probes and every context/inventory requirement. At original exec
stop the namespace owner additionally retains its own proc fd directory as a
readonly NOFOLLOW/CLOEXEC directory with original device/inode and procfs checks.
After hardening enumerate that original directory directly, not by reopening a
pathname or /proc/self/fd alias. This is live inventory of the original task,
not a saved name list. No FD/acceptance/PID is exported or adopted.

Bound getdents64 to a 512-byte buffer, three reads and eight distinct records;
reset its private directory offset to zero for each observation. Validate complete
aligned Linux64 records and NUL-terminated dot/dotdot or decimal descriptor names,
refusing duplicates, unsupported types/lengths, extra entries or missing EOF.
Require exactly dot/dotdot and stdio0/1/2. Keep task enumeration and all other
live context/mapping checks unchanged. Kernel proc fd iteration uses its original
task reference, as in the [Linux6.6 fd implementation](https://raw.githubusercontent.com/torvalds/linux/v6.6/fs/proc/fd.c);
local tests must prove actual behavior after ND0 and reject missing/foreign/
weakened retained directories. Unsupported behavior refuses without privilege.

Add decoder bounds/mutation tests and original-directory post-ND0 inventory versus
failed fresh open. This changes only fixture observation plumbing, not helper,
64/512 policy, hardening order, host state, qualification or authority boundary.
