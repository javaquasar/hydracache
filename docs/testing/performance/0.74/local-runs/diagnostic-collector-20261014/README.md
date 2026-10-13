# Local collector EOF and process interruption evidence

The bounded pipe-to-file writer is implemented and locally exercised. Only a
nonroot private fixture entry is enrolled. It cannot become a root producer,
start an installed unit, revoke a cgroup or acknowledge lifecycle completion.

Baseline is `802d1a20ea66d94df219a8c787803e82fe58e950`; architecture and process-loss
preregistrations are `0700f946` and `84fc6642`. Implementation is
`186d30bab68ce541050c8c638c047bdc49021086`. The
[contract](../../diagnostic-collector-design.md) retains the approved root-owned
collector architecture and lists the production prerequisites still to connect.

## Actual writer behavior

Worker-facing descriptors are pipe write ends, blocking and close-on-exec before
Stdio transfer. The original collector retains nonblocking readers and private
evidence-file descriptors. One pump quantum is at most 8192 bytes per stream.
Tests exercise binary fragmentation, both-stream progress, exact frozen 8 MiB
ceilings and overflow. Overflow retains a bounded prefix, latches refusal and
never returns a drain receipt. Creation is exclusive even under eight concurrent
fixture calls. No failure truncates, unlinks, retries or repairs retained files.

Both streams must return an actual zero-length read after their queued bytes
drain. An owned cat is actually killed with SIGKILL while the parent retains
another copy of its stdout pipe writer. EOF remains absent, the copy appends
late bytes, and only closing it permits exact drain/hash reconciliation. This is
a kernel pipe observation, not original production-cgroup revocation.

After EOF the collector syncs its files, makes them read-only, rereads the retained
objects in bounded chunks and verifies byte counts/hashes against incremental
stream hashes. Named ancestry/stream checks bracket the operations. It syncs the
directory and explicitly closes descriptors before returning consumed fixture
drain data. Reopened files cannot recreate that owner or acknowledgement.

## Retained development failures

The missing-module red test returns 101. The first functional run passes 13
tests and fails a same-length content-mutation test: metadata-only detection
missed the rewrite on the WSL workspace filesystem. Retained-file byte hashing
now refuses this mutation even if timestamps are unchanged. The allowed byte
ceilings and refusal policy are not weakened.

The first process matrix exits before DirectoryCreated; its initial root
preparation did not explicitly require private mode. The corrected harness sets mode 0700 before
spawn and preserves the unchanged root policy; it also retains child stderr for
useful failure diagnosis. A strict lint run rejects a test's needless range loop;
the corrected test uses an iterator without a lint exemption. A broader test
invocation names a nonexistent diagnostic_builder target and is retained as a
command-selection failure, not a failed product test; the corrected invocation
uses existing reviewed integration targets.

## Process interruption and checks

Three exact-source fast repetitions pass 20 tests each, with the manual proof
skipped. Three manual repetitions each pass 15 SIGKILL checkpoints including
after-return loss, one uninterrupted control and 32 fresh refusal inspections.
The parent owns every original child, verifies signal-nine death, snapshots
retained bytes and metadata before/after, and reaps all helpers. Fresh inspectors
cannot create a replacement collector; snapshots remain unchanged. Creation,
append, sync, permission, close and acknowledgement barriers are covered. Logical
fault injection and an actual write error are separate tests. Neither family
proves power-loss durability or root worker isolation.

Linux library tests pass 624 with 12 manual proofs skipped; six reviewed
integrations pass 66, and Linux doctests pass 57. Windows package tests pass 204;
root performance/gate/evidence/governance contracts pass 166. Current and MSRV
checks, strict lint/Rustdoc, scoped formatting, gate registry, documentation,
book and local performance-contract checks pass. This is not a full release,
merge or tagging gate.

Qualification dry-run and require-ship retain expected exit-one refusals; no
qualification artifact is created. Product source, old unit/stream bindings and
frozen numerical inputs remain unchanged. No server operation or performance
claim occurs. Collector hashing/sync overhead requires later admitted-host
measurement before this architecture may support product evidence.

## Reproduction

Run `C:/Python313/python.exe docs/testing/performance/0.74/local-runs/diagnostic-collector-20261014/verify_packet.py`
for read-only exact-source/hash verification. The Linux manual command is
`cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --locked --lib
diagnostic_collector::crash_tests::collector_owned_sigkill_matrix_retains_without_replay
-- --ignored --exact --nocapture --test-threads=1`. It needs nonroot Linux,
uses a fixed 180-second gate and 30-second per-helper harness deadline, and must
run single-flight without relinking libtest.

Next connect trusted per-packet kernel sender/time and independently enrolled
issuer/digest/epoch pins to a separately signed collector start projection.
Then implement the private root constructor, authenticated original start,
autonomous watchdog and exact retained-cgroup revocation/empty proof. Only the
joined evidence may drive completion, cells two through four and host rehearsal.
