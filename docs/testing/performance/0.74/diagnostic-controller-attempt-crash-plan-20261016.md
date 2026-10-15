# Original controller attempt process interruption

Baseline `2616276735c16e527640cab98350c1815b7bd1c6`. Test the existing
original-controller request retention path without changing its runtime API,
storage format, guard deadlines, qualification inputs or execution permissions.
The earlier logical fault matrix and legacy process proofs do not establish
process-loss behavior for this composition.

Add a private Linux nonroot unit-test subprocess harness under the OIDC original
fixture. Keep synthetic controller, worker, OIDC and authority fixtures below a
parent-owned mode-0700 temporary root by supplying the child TMPDIR. Preserve
the strict modeled authority rule: its separate parent-owned mode-0700 temporary
leaf stays directly under /tmp. A private fixture helper supplies that original
authority path, without changing the default helper or runtime opener. The kernel packet uses the child's actual
SOCK_SEQPACKET message credentials, pidfd, host context and clock observations,
with the child itself as sender. This is not independent production enrollment
or a separate external controller process.

Use seed `0x0740c095` to permute the eight existing AttemptStage checkpoints and
an AfterReturn checkpoint. At each checkpoint the original common fence and
controller/kernel/authority chain must still be held. Signal readiness through
a bounded private stdout protocol; the parent verifies original flock exclusion
before SIGKILL of its exact owned child and requires signal 9 on wait. Include
one uninterrupted AfterReturn control. No sleep-based correctness decisions,
external process discovery, service operation or product workload is permitted.

BeforeCreate leaves an empty unadmitted leaf. Created leaves an empty exclusive
file; PrefixWritten leaves incomplete canonical data; later cuts leave complete
bytes, with mode 0600 before sealing and 0400 after. Verify the exact leaf
inventory, bounded bytes, ownership/link count and canonical document fields.
Retain snapshots of bytes, inode, mode, size and modification/change stamps;
SIGKILL and two separately started read-only inspections must not alter any
fixture context. Healthy control may dispose its private trust fixture roots,
but must preserve all fixed intent/campaign bytes and metadata.

After termination the same original flock must be available. Each fresh process
must accept only BeforeCreate as empty, and reject every retained partial or
complete attempt as fixed-leaf Conflict without adoption, deletion, repair,
acknowledgement reconstruction or retry. The AfterReturn case discards the
runtime acknowledgement: readable bytes must not restore it. No durable
authority refusal, global nonce ledger, host rollback, reboot, power-loss,
original start, writer revocation or later cells are proved.

First capture the ten-case original baseline. Add a test-first missing-harness
gate, then implement the manual matrix and ordinary protocol parser negatives.
Register the ignored gate explicitly, preserving existing gate commands. Run
three independently started current-Rust matrices and one MSRV matrix serially,
without concurrent heavy builds. Recheck affected original trust/fence tests,
format, check and strict Clippy, static contracts, registry, docs and local
closed-admission gates. Retain failures as well as final results in a separate
evidence packet, then update the design, checklist, README and article.

Only the new local joined SIGKILL boundary can close. Server, GitHub, actual
keys/tokens, product measurements, qualification, push and release admission
remain outside this slice.

## Fixture origin correction before acceptance

The first development run fails before its checkpoint because the existing
authority fixture correctly refuses a nested TMPDIR origin. Retain this negative.
Do not loosen that opener: use the separate parent-owned /tmp authority leaf
above and include its complete byte/metadata inventory in every crash/control
and fresh-reader comparison. This is test fixture plumbing, not an admitted
change to the authority namespace.
