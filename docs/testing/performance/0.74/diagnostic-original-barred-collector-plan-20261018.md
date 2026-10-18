# Original barred collector enrollment

Baseline: 0cba14bb08f9d302a7b6646f505420bced6574d1. Pending custody has
three descriptors and survives selected owner/notice loss, but no process owns
the original readers/sinks independently of the preparing owner.

Add a private typed join that enrolls a second, barred child before one unchanged
closing full validation frame. Do not compose another recursively validated
descriptor-plan or pending-custody constructor. Construction and revalidation
must each retain the full 34 authentication/signature checks and immutable
Reserved/Starting/progress/challenge origins. No deadline reset or weaker leaf
validator is allowed.

The collector inherits exactly the original fence alias, original alarm, two
readers, two empty O_RDWR sinks and its private seqpacket endpoint. It inherits
no pair/message writer, unsent message, standard stream or other descriptor.
Its atomic CLONE_PIDFD identity is owned before parent signal restoration can
fail. Reuse the exact existing custodian cancellation primitive, not PID lookup
or PID signalling. The old generic O_WRONLY engine and pending custodian kernel
remain byte exact.

The collector accepts no command, reads no stream and writes no sink. Its first
B notice means only barred descriptor isolation, never authenticated readiness.
Expiry, unexpected peer activity/loss or polling failure closes all four I/O
descriptors before a best-effort E/L/F notice, then parks retaining the fence.
Any close error retains custody and cannot become a quiescence/completion proof.
Holding its own fence alias prevents early unlock if the preparing owner and
the separate custodian die while collector sink rights still exist. Parent
cancellation stops/reaps the collector before dropping the exclusive original
borrow or custodian. Failure objects retain both processes and the original
borrow until explicit ordinary cancellation.

Declare opt-in original healthy, refusal and process tests in
diagnostic_original_barred_collector_tests.rs. Check exact child FD inventories,
unchanged records/offsets/origins/mask, full frame counts, sticky refusals,
collector/custodian death and autonomous I/O quarantine on expiry and peer loss.
Check the fence remains busy after exact custodian SIGKILL and becomes kernel-
available only after exact collector SIGKILL/reap; durable history still refuses.
Run a healthy screen first without tracing or deadline changes. Use current and
MSRV Linux binaries in private nonroot synthetic fixtures on the dedicated host,
with pre/post guards, and focused fast/static tests, formatting, checks, strict
Clippy, registry/docs and local non-promotable performance contracts.

Retain failures and distinguish fixture mistakes from implementation failures.
This selected barred-only matrix is not the complete active writer/owner SIGKILL
matrix. Authenticated activation, original send/start, worker tree enrollment,
writer revocation and completion remain closed. No installation, supervisor or
clock mutation, product load, qualification or performance claim is authorized.
