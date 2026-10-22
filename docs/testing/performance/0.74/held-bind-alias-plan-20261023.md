# Retained read only alias of the original sealed launcher

Baseline is 17b9c689. The user approved a read-only bind alias of the original
sealed launcher with mount retention through retirement. Preserve every old
validator/frame/retirement body and the original absolute 300/60/10 windows.
No clocks, services, supervisor, installation, server or product workloads.

Use open_tree with AT_EMPTY_PATH on the retained launcher itself. Apply readonly,
nosuid and nodev to the detached mount before one move_mount publication. No
source PID path, copy, replacement executable, alternate syscall fallback or
chmod of the original. Require root-owned immutable executable source metadata,
the original expected identity/hash, and retained safe directory/namespace pins.
The fixed protected parent is /run/hydracache-perf/held-exec-074, which must
already exist; runtime does not install it. Exclusively create attempt-hex/launcher.
Require the same current and PID-1 mount namespace for production so the mounted
alias is not destroyed solely by creator loss. The original manager/full frame
remains authoritative; this namespace check alone is not manager enrollment.

A new consumed wrapper adds this alias under the existing MaterializedHeldUnit.
Unchanged full material validation brackets creation and fresh alias observations.
Pin namespace, ancestor/attempt directory, original inode and mount ID/flags;
fresh reopen must detect replacement, unmount/remount and source metadata drift.
Failure retires the same owner, never retries or exports an address/FD capability.
The private address uses the fixed activation-v1 entry already implemented.

No Drop or refusal may unmount a possibly published alias. The kernel-mounted
alias survives closed Rust/source FDs in the retained manager namespace. Existing
root guardians retain the original fence; this change cannot prove all writers
stopped and therefore exposes no cleanup/completion/unlock operation. Keep partial
attempt directories/mounts after uncertainty. Future cleanup needs the actual
writer-cessation/retirement proof, not a caller boolean.

Local fault tests use only root in a fresh private mount namespace and unique
temporary directories. An external namespace harness survives each killed
creator, preserving the namespace; test-only cuts precede/follow detached clone,
attribute setting and publication. Check original inode/content, readonly writes,
actual synthetic ELF execution, source FD closure, Drop, SIGKILL, duplicate target,
metadata/namespace/path/mount substitutions and sticky refusal. Test cleanup
occurs only in the disposable namespace after exact children are reaped; it is
not production retirement evidence. Record the first matrix result without retry.

First record a red current-source gate. Then run nonprivileged model/refusal
tests, GNU 1.94/MSRV 1.88 compilation/lint and the isolated local mount matrix.
If the local kernel lacks the required mount capability, retain that refusal and
leave the real gate red; do not contact a server or change services to obtain it.
Register the new runtime-visible layout and manual tests without altering old
formats or qualification. No original delivery or healthy admission is claimed.
