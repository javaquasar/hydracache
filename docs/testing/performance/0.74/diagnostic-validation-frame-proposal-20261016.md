# Original validation composition proposal

The publication-free budget experiment blocks the straightforward durable
Starting implementation on this local host. All twelve chains expired before
finishing nineteen unchanged checks, including the six normal release-profile
chains. No record was published and no process was started. Increasing the
deadline or replacing a failed chain is not an acceptable repair.

## Decision requiring architectural review

Consider a private validation frame for one before/after effect boundary. It
would check the complete original live inputs without recursively multiplying
the same leaf checks through every wrapper. Immutable signature/grammar proofs
could be retained only while their exact original bytes, verifier inputs and
owned projections remain identical. This changes the composition and sequencing
of security observations, not just the implementation of a signature verifier.
It therefore needs a separate reviewed design before runtime implementation.

The isolated owner diagnostic observes 0, 1, 2 and 4 IPC authentications per
kernel-worker, signed-controller, reviewed-build and OIDC check respectively.
The first Rust 1.94 dev sample's single refusing consent check performed 264 IPC
authentications. These counts support investigating composition. They do not
attribute all elapsed cost to signatures, prove an exclusive kernel owner or
establish a safe alternative bracket.

## Required invariants for a candidate

- Enter only by consuming the already admitted original owner. No constructor
  from bytes, caller peer/time/key/path, reopened origin or replacement owner.
- Keep actual sender liveness, credentials/groups, namespaces, kernel time and
  original clock origins live. Never cache those observations across effects.
- Check original worker/controller/OIDC trust objects, authority epoch/history,
  retained descriptors, names, bytes, metadata and exact storage inventory on
  both sides of every create, write, seal, sync and acknowledgement boundary.
- Retain the original challenge/Heartbeat/consent windows, Reserved origin and
  total lease. No re-anchoring, silent handoff or new lease is introduced.
- Keep sticky cross-refusal and original fence ownership through every failure.
  No retry, repair, overwrite, auto-adoption, recovered acknowledgement or global
  replay claim follows from a successful byte check.
- Leave existing public guards and their predicates intact. Register only a new
  private consuming path, with narrowly checked historical restoration adapters.

## Review and falsification before publication

First enumerate the leaf predicates, their required relative order and every
effect they protect. Identify which repeated work is truly immutable and which
live observations cannot be coalesced. A frame must not merely set a cached
"validated" bit or reuse a previous boundary's observation. Keep a complete
functional reference for immutable proof checks and seeded bytes/key/projection
mutations. Add live trust/epoch/namespace/sender/storage drift and concurrent
mutation canaries between every proposed observation cut. Retain each negative
result; do not infer correctness from a timing pass.

Only after that review implement one isolated candidate and rerun the unchanged
budget fixture on both compiler/build lanes, reporting all samples. Then add
real publication I/O, signed transport delay, storage adversaries and the exact
SIGKILL/lost-acknowledgement/restart matrix. Completing the publication-free
fixture alone is not sufficient margin or execution admission.

## Remaining sequence

Durable Starting and its crash proof precede authenticated executable/output/
process/cgroup start. Root collector ownership, autonomous watchdog and proven
cessation of every writer precede completion and later cells. Integrated local
gates then precede exact signed installation and a bounded nonproduct server
rehearsal. Product pilots, six-hour/24-hour qualification, changes to frozen 0.73
artifacts and release admission remain outside this proposal.
