# Signed local worker policy evidence

This packet proves local policy-byte verification under synthetic external trust
and asserted context. It implements neither local account-file inspection nor
real issuer/host enrollment. See the [design](../../diagnostic-worker-policy-local-design.md)
for the approved model, canonical signature bytes and remaining Linux work.

Windows baseline passes 30 tests (15 library and 15 artifacts). The missing
module check refuses E0432 with observed Cargo native exit 101. First green has
12 new cases; reviewed coverage adds the oversized/key/pin revalidation case.
Final Windows scope passes 43 (15+15+13); Linux passes 269 (222+29+2+3+13), with
one existing system-bus test ignored. Three further Linux runs pass all 13 cases.
Seed 0x7512026 drives 256 valid re-signed mapping/document-digest mutations in
each repetition. Each mutated policy first verifies under its own matching pin,
then refuses under the original pin; invalid parsing cannot explain this result.

Independent guard threads use synthetic keys and context, not namespace changes
or real accounts. Group tests preserve the exact supplementary list and bounded
primary union. Trust/context/envelope changes refuse without adopting replacement
values; restoration cannot reset a refused original. Same-policy revalidation is
idempotent and confers no launch authority. Stale pins supplied to a new guard
remain a separate durable enrollment problem.

Root contracts/evidence/governance pass 128 checks. Package checks/strict lints
cover Windows and Linux; the root guard's later constant assertions have separate
reviewed xtask check/lint captures. Documentation, local performance contract,
links/book and format gates pass; require-ship is an expected native exit 1.
The packet records working-tree local checks, not clean-source qualification.
Pending packet registration during root tests is followed by a separate final
registry check; registry presence is not host or cryptographic packet admission.

`python verify-packet.py` checks exact raw captures, SHA-256/byte sizes, test and
seed rows, closed proof boundaries and the frozen qualification manifest. It
does not authenticate simultaneous replacements of captures and manifest.
No host mutation, workload, performance measurement or full workspace milestone
verification ran. Test compiler timings are not cache throughput measurements.
