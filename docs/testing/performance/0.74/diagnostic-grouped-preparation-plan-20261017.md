# Grouped nonexecuting collector preparation experiment

Baseline is 67419018a60843da0825324f2cb82e67d6c12ddc. The rejected separate
material/message/alarm composition and its original Causal(Time) result remain
retained. The latest request to continue follows the explicit proposal to group
only those nonexecuting steps; this experiment does not authorize start/send,
production enrollment, installation, service changes or product workloads.

## One hypothesis and its boundary

Construct retained material, an unsent FD-bearing message and the absolute alarm
inside one opening/closing original frame instead of three separate brackets.
The candidate exists only under cfg(test). Do not change existing runtime
validator bodies, public APIs, original time origins, thresholds or their tests.
Remove four intermediate complete rounds in the candidate schedule only.
Acknowledged output creation, Starting publication and their effect-local rounds
stay unchanged. This experiment cannot miss a persistent mutation at closing;
it does not promise detection of transient drift restored between endpoints.
That reduced observation frequency is the explicit architectural tradeoff.

Opening uses the existing material_round on the acknowledged original outputs.
Retain the original projection/Starting and load the signed nonexecuted fixture
bundle through the existing artifact inspector; reuse material.bind, worker,
message and inspect_message. Derive a provisional alarm from original Reserved,
Starting and progress kernel-receive origins through existing boundaries and
KernelPacket time sampling. Arm the existing one-shot BOOTTIME alarm. Closing
uses the unchanged original_round, which repeats the complete original frame,
bundle/build, state-transition, message/FD and original deadline checks. Require
its bounds to equal the provisional bounds, then inspect the alarm again.
Only then mark the private material ready. Any failure latches the original
chain and retains the whole owner/fence until failure drop. No outgoing IPC.

## Tests before acceptance

First run one Rust 1.94 normal GNU release healthy screen on the isolated server
with the same retained signed bundle data, never executing it. Stop that cohort
on failure; do not convert a negative characterization into healthy acceptance.
If green, run three fresh healthy samples per compiler, including MSRV 1.88.
Require five ordered internal checkpoints, exactly 34 IPC authentications and
signatures for grouped construction, complete unchanged reference revalidation,
unchanged intent/Reserved/Starting records, fence retention and refusal of fresh
context after owner drop. Assert absolute offsets 300/60/10 seconds from their
original origins, not a refreshed or later Starting controller timestamp.

Exercise persistent trust mutation after each internal checkpoint; content and
inode bundle drift; output writes; projection, state, writer, message and alarm
tampering; and original expiry without clock adjustment. Check the precise
refusal family, sticky refusal on further validation, unchanged durable records
and held fence through failure drop. Do not treat a late timing failure as proof
of a requested earlier binding fault. Keep all existing kernel/reference tests,
seeded bounds, static byte-exact runtime guards and compile-fail boundaries green.

Use a new nonroot 0700 temporary server root with exact boot/supervisor/artifact/
receipt/context guards and independent executable-object postflight. No root
mutation, installation, supervisor stop, observer, product load or expensive CI.
Retain exact source capsule, binary digests, raw traces, results and verifier.
Run current/MSRV checks and strict Clippy, targeted integrations, static contracts,
registry, format, docs and local non-promotable performance contract.

## Decision after the screen

A healthy fixture can justify further crash/ack tests of this candidate only;
it does not make the one-shot alarm an independent guardian or authenticate
original start. Production adoption, autonomous guardian/writer revocation,
completion, later cells and their full SIGKILL/lost-ack matrix remain separate.
On failed healthy or fault admission retain the negative result and leave the
production path unchanged. Never enlarge a deadline to make the candidate pass.
