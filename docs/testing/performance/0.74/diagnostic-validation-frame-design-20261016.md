# Original consent validation frame design

The operator approved the validation composition refactoring after source
47372f9f. Implement one separate consuming candidate before durable Starting.
The retained twelve-chain baseline remains a failure, not a retry to be erased.

## Candidate and ownership

Consume an already admitted OriginalStartConsent into a noncloneable framed
consent owner. A public consuming adapter may expose that owner for observation;
the frame implementation, fields, phase driver and fault hooks remain private.
There is no constructor from data, extraction, retry, start or publish API.
Construction failure retains and refuses the complete original chain and fence.
The existing public guard implementations remain unchanged.

This first candidate coalesces only the outer recursive orchestration. It does
not cache signatures, grammar, live observations or a validated flag. Reuse the
existing original Reserved live check, progress authentication, causal binding
and challenge window, original worker/policy APIs and consent authentication.
In particular, keep the complete original OIDC/controller/build/worker guards
inside the Reserved live check. A separate immutable-proof optimization is not
bundled into this candidate.

## Predicate inventory and order

Each validation performs a working round followed by an unhooked closing round.
Every subsequent validation repeats both rounds from the original owners; no
previous round can authorize the next boundary.

| Step in each round | Existing predicate retained |
| --- | --- |
| Storage before | Original fence, ancestry, file descriptors, names, metadata, bytes and exact two-file inventory |
| Reserved live | Two original request captures, current authority binding, exact Reserved encoding, kernel sample, signed collector requirements, original request/state/controller identity and signing interval |
| Progress | Same original receiver, actual sender credentials/liveness, ordered original samples, original Heartbeat age, signature/principal, operation/revision/identity/controller/source/preset/id/nonce and anchored total lease |
| Causal | Exact original challenge/nonce/digests/identity/issuance, response nonce and original challenge window |
| Prepared | Original point, current policy/epoch, signing window and exact original Reserved-to-Starting transition |
| Consent | Unchanged original kernel consent authenticator and equality of the entire frozen signed body |
| Storage after | Reinspect the same original fence and complete inventory after authentication |
| Final clocks | Fresh original kernel observation; exclusive challenge, Heartbeat, consent and total ages, original consent Unix window; advance only the observation cursor, never an admission origin |

Prepared checks use the same conditions as the old preparation revalidator,
without recursively invoking its causal wrapper. All other semantic/signature
checks call existing implementations, not new alternative parsers. The final
clock check strengthens the closing observation; no clock origin or duration
is replaced. This is sequential evidence, not atomic or continuous revocation
proof against a compromised root/kernel.

## Tests and comparison

First retain a missing-adapter compile failure. Add a candidate copy of the
publication-free three-chain/nineteen-check fixture, changing only the consuming
adapter and diagnostic labels. Keep the original baseline fixture byte-exact.
Run both named dev/optimized-crypto and normal release lanes on Rust 1.94/1.88;
retain every refusal. No numerical product claim or new threshold is introduced.

Test current and minimum compilers, seeded exclusive clock edges and unchanged
leaf predicates. Cover actual trust/authority/storage drift at every offered
working-round checkpoint and a concurrent mutation control, then require the
closing round to detect it and retain cross-refusal/Busy ownership. Fault hooks
are test-only and never occur after the closing round. Also compare unchanged
reference and framed refusal behavior on independent fresh original chains.
Require negative clone/deserialize/extract/retry/start/publication APIs and
exact narrow historical restoration, including mutation canaries.

After the focused gate passes, commit the candidate and its evidence separately.
A release-profile budget pass permits only work on real Starting publication;
its actual I/O, signed transport, adversaries and SIGKILL/lost-acknowledgement
proof must still pass. Root output/authenticated process/cgroup start, watchdog,
all writers, later cells, full local gates, signed installation and bounded
nonproduct rehearsal follow in that order. No product workload is admitted.
