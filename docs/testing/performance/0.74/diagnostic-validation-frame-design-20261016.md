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

## Candidate observation, not admission

The local implementation consumes the original owner and performs the two
rounds without caching signature results or live observations. The original
baseline fixture and lower runtime sources remain unchanged. The new static
adapter restores only exact module registrations and comparison adapters;
unapproved-tail and wrong-module canaries still refuse.

The preregistered dev/optimized-crypto lanes completed 6/8/11 checks on Rust
1.94 and 10/10/11 on 1.88, all short of nineteen. Normal release completed
19/19/5 on 1.94 and 19/19/19 on 1.88. The short release chain refused on the
unchanged kernel clock-order guard: realtime moved backwards by about four
seconds while boottime advanced. These observations are not product speedups,
independent product benchmark pairs, installed-binary proof or an all-green
publication budget. The fixture's `admitted=true` label means nineteen
publication-free checks completed, not execution or server admission.

The first release live-drift matrix retained five cut-zero refusals and then
failed during construction of the next frame on another backwards realtime
step. It did not reach the remaining cuts, reference comparisons or concurrent
control. Read-only local observation also found realtime steps relative to
uptime; WSL timesyncd is enabled while Windows W32Time is stopped. This is a
local clock-environment blocker, not grounds to change the guard. No time
service or host clock was changed. Any environmental control must be explicitly
declared, preserve these failures and run a separately labelled complete cohort;
it cannot turn a discarded attempt into a pass.

The subsequently operator-approved [WSL clock control](local-runs/validation-frame-clock-control-20261016/README.md)
also refused before tests: realtime moved backwards at preflight sample seven
despite the exact time service being inactive. The service was restored and
independently checked active/enabled. No matrix or budget test began in that
control. Stopping timesyncd alone is therefore insufficient here; no clocksource,
Windows clock/service, deadline or runtime guard was changed. Complete fault
coverage still needs a separately approved stable environment, not an identical
retry or an unsupported claim that NTP was the unique cause.

## Dedicated Linux fault control

The operator separately approved running only the unchanged fault matrix in an
isolated temporary directory on the dedicated Linux server. The
[retained control](local-runs/validation-frame-server-fault-20261016/README.md)
passes forty mutations, five old-reference cases and the healthy-order,
construction and concurrent controls on each compiler, once per lane. Installed
binary/receipt hashes, supervisor PID 8839/start time/NRestarts zero and boot
remain exact. No installation, supervisor stop, product work or clock change
occurred. Source 7b799270 commits only the nine frame source/test/contract files;
the following Starting byte slice is separate. The earlier WSL failures and
dev budget negatives remain. Actual publication and crash boundaries still need
their own proof; this is not production worker enrollment or server admission.
