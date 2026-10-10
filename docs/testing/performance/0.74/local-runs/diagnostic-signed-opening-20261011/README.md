# Local signed-context credential status opening

This is local working-tree safety evidence, not enrollment, product performance
or qualification. See the [design](../../diagnostic-signed-opening-local-design.md)
and [manifest](manifest.json). Raw captures are copied byte-for-byte without
normalization or omission; ordinary missing-API refusal and expected-red ship
admission are retained. Git treats raw logs as binary-preserved text.

Opening order is account/open/account/credentials/account. The private opener
observes no credential projection before the post-open signed account/context
check. Numeric assertions come only from the original signed mapping, with one
bounded supplementary-list clone. Later checks never reopen original objects.

Linux passes 340 with one existing system-bus ignored; Windows passes 43.
Root contracts/evidence/governance pass 133. Fifteen new opening tests plus one
private opener test pass three extra repetitions. Seed `0x7572026` supplies 256
modeled failure positions per run. Actual owned helpers and synthetic signed
account documents are not real issuer/fixed production inspection.

The post-open mutation is a private deterministic seam at the real status-opening
boundary; gate failures are modeled stages. Neither is exhaustive kernel race,
file-opener credential, worker mount, all-thread or initial-host attestation.
Durable epoch/refusal, production preparation and original authenticated start
remain closed. No host workloads or performance measurements were run.

Run `python verify-packet.py` here for read-only inventory, hash, test-result,
closed-boundary and frozen qualification consistency checks.
