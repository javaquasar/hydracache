# Autonomous watchdog and writer fence

Baseline 5621df7b4a5a7f7e5da0f77c7a6459a7d5e04fbf has an absolute timer
inside the nonexecuting preparer, not an independent process. Its SIGKILL packet
proves preparer cleanup only. The operator requests an autonomous watchdog and
fence retention through writer cessation without installing a release, stopping
the supervisor or executing a product workload.

## Implementation boundary

Add a private Linux kernel guardian primitive below watchdog, separate from the
unchanged portable progress model and original admission chain. No public
constructor, CLI, caller authority input, service dispatch or start permission.
Production joining must later transfer sole ownership from original checked
objects; generic private descriptor inputs are not authenticated enrollment.
Do not change existing validators, observation frequency or 300/60/10-second
origins. Accept only an already selected absolute CLOCK_BOOTTIME deadline;
never renew it on process creation, readiness or controller loss.

Create a separate process before acknowledging readiness. Prepare all allocation
and descriptor lists before fork; the child uses only bounded stack storage and
async-signal-safe syscall operations, never Rust destructors, allocation, logging,
mutexes, environment access or library validation after fork. Close all inherited
descriptors except the explicit retained set with close_range. Fail closed if
descriptor isolation fails. Do not use parent-death SIGKILL: it would release the
very fence that must outlive the controller.

The guardian exclusively owns the fence open-file description, both pipe readers,
both regular-file sinks, an absolute timer, controller pidfd and pinned writer
pidfds. It has no writable pipe aliases and no public descriptor export. Merely
duplicating the current original Fence is forbidden: that owner's explicit
flock unlock affects duplicates. The original fence implementation stays intact
until a separately checked ownership transfer is available.

Controller exit or absolute expiry triggers SIGKILL through retained pidfds only.
Retain the fence until every pinned writer is terminal, both actual reads report
EOF, and bounded sink writes, fsync and explicit close succeed. A hidden pipe
writer must prevent EOF and therefore release even after all known pidfds exit.
Process terminal readiness alone is not all-writer cessation. Overflow, I/O,
isolation or signal errors enter a sticky retained-fence state; no completion,
adoption, retry, resource cleanup or new deadline follows. Guardian SIGKILL or
host loss requires durable-history exclusion and external supervision; an
in-process flock cannot survive the death of its last kernel owner.

## Declared checks

Fast units cover deadline arithmetic/equality, exact ownership inventory and
release conjunction. An ignored isolated synthetic process matrix covers normal
EOF, real controller SIGKILL, SIGSTOP through absolute expiry, a stopped writer,
an unlisted duplicate pipe writer, readiness ACK omission, and sink failure or
overflow. Use pidfds, wait statuses, independent flock contention and exact
retained output bytes as oracles. No product binaries run. Record all negatives;
do not change an expiry or observation to obtain a pass.

Build and lint with current Rust and MSRV on Linux; run the manual matrix on an
authorized nonroot host scratch root with pre/post supervisor/receipt/campaign
guards. Run existing static contracts, registry, formatting and documentation
checks. Commit the implementation separately from evidence and article updates.

This stage can close the isolated guardian mechanics, not authenticated original
send/start, cgroup enrollment, production root-sink transfer, durable completion,
cells 2 through 4 or server execution admission.
