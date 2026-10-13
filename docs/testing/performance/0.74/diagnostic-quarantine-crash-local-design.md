# Process loss during owned Starting quarantine

Baseline is `b6c76476be7b99117a267dfcd570e4dffa8c7e00`. This local P1 proof
tests the existing consuming quarantine at actual authority refusal write
boundaries. It changes neither the public runtime API nor authority encoding,
write order, sync calls or launch/cleanup permissions.

## Private instrumentation and exact owned process

Add a scoped thread-local checkpoint observer compiled only under cfg(test).
Invoke it at the existing four authority publication fault checkpoints. It
observes and blocks; it never changes the selected fault, performs writes or
reconstructs a guard. Reject nested installation, restore on return/panic and
prove thread isolation. Production builds contain neither hook nor observer call.

Extend the existing private nonroot subprocess harness with a quarantine phase.
Retain the original attempt, immutable Reserved anchor, cell-one pair and
acknowledged Starting record. Block before invoking quarantine, at RecordCreated,
RecordSynced, MarkerCreated and DurableCommit, or after successful API return.
The parent owns the exact child and helper pidfd; SIGKILL targets that child only.
Common-fence contention precedes death; release and exact helper termination
follow it. Do not relink the test executable while subprocess proofs are active.

## Retention and fresh observation

Check the original five attempt files, initial authority pair and exact tail
shape/mode/hash at each boundary. RecordCreated is an empty 0600 record;
RecordSynced is sealed without its marker; MarkerCreated adds an empty 0600
marker; DurableCommit and AfterReturn retain a sealed validated refusal pair.
BeforeRefusal keeps the original active authority, but retained attempt context
still prohibits fresh reservation. No missing PID or cgroup inode proves cleanup.

After each death/control, two fresh processes inspect reservation eligibility
and the authority journal read-only. The test parent supplies the original fixture
policy digest, with the known fixture issuer and current local host/namespace
context. This is fixture observation, not independently enrolled production trust.
Expect active revision one before refusal, invalid partial journal at the first
three write boundaries, and refused revision two at durable commit/after return.
Never append, repair, retry, delete or recover a runtime acknowledgement. Retained
bytes and metadata must be unchanged after death and both inspections.

## Preregistered gates and limits

Capture baseline quarantine tests and a missing-hook red test. Run fast hook
scope/panic/concurrency checks and the eight existing quarantine tests. Register
one ignored/manual proof with unchanged seed 0x074d and 600-second harness gate.
Six SIGKILL cases plus one uninterrupted control and fourteen fresh inspections
must pass in each of three same-source repeats. Run all seven previous process
proofs separately as regression, using single-flight bounded scheduling.

Run scoped Linux/Windows regression, current/MSRV checks, warnings-denied lint
and Rustdoc, formatting, docs/book and local performance-contract checks. Retain
all captures, source pins and frozen qualification inputs in a read-only verified
packet. Process loss is not host reboot or physical power-loss durability. No
product workload, server operation, qualification or shipping is admitted.
