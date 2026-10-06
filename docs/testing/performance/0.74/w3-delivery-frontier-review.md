# W3 delivery-frontier feasibility review

Status: local semantic investigation, not a D2 authorization or a product candidate. The existing
W3 rejections, W10 composition ledger and qualification boundary remain unchanged. This review
screens possible new mechanisms before product implementation or performance measurement.

## The boundary that must be preserved

The retained adaptive attempt committed two SETs while the first response's write was blocked;
the canonical path permits one. Source inspection identifies the stronger boundary:
`serve_connection` executes one command and awaits `write_response`, which awaits both
`write_all(encoded_reply)` and `flush()`, before dispatching the next buffered command.

"Delivery completion" here means completion at the supplied `AsyncWrite` boundary. It does not
mean peer receipt, TCP acknowledgement, durable persistence or an empty kernel send queue. A new
implementation cannot silently replace that existing boundary with application-buffer acceptance.

The first SET may already be visible while its reply is blocked. The second SET may not be visible
until the first reply's complete write **and** flush succeed. First-byte progress, a writable hint
or a full write with pending flush is insufficient. Other connections must continue to observe the
exact committed state and make progress; native/store cost cannot be hidden behind a held batch lock.

## Deterministic verification

Baseline source `8bf4eb26` passed the four existing adversarial transport tests and three adaptive
regressions before test changes. The added fixture supplies two SETs and QUIT in one read. A gated
writer admits exactly a selected prefix of the first `+OK\r\n`, then returns `Pending` without
waking itself. The test explicitly polls the serving future to that boundary; it does not sleep
or infer correctness from elapsed time. Timeouts bound completion after release only.

| Accepted first-reply bytes | Blocked boundary | Visible SET mutations |
| --- | --- | ---: |
| 0 | first write | 1 |
| 1, 2, 3, 4 | remaining suffix after partial write | 1 |
| 5 | flush after the entire first reply | 1 |

A separate connection reads the first value and a miss for the second key in every case. After
gate release, the original connection returns three exact ordered `+OK` replies and has two SET
mutations. A second fixture fails the first flush after writing its complete reply: the serve call
fails loudly, the first mutation remains visible, the second key remains absent, and no second
reply or automatic retry occurs. Existing fragmentation, short-write, large-reply, disconnect and
pipeline-one checks remain intact.

The gate's default zero-prefix behavior preserves the older closed-write fixture. New helper
controls live only in the integration test file. No parser, server, encoder, native store or public
API is modified. The finite prefix enumeration covers every cut of this five-byte SET response;
it is not a proof for all commands, transports or response sizes.

## Mechanisms screened before candidate implementation

- **Writable/readiness preflight:** does not prove that the complete previous reply and flush have
  succeeded. It cannot grant permission for another mutation under the generic writer contract.
- **Write a prefix, then collect later replies:** fails at cuts 1 through 4; a progress signal does
  not release the mutation frontier.
- **Write the complete reply, postpone flush:** fails at cut 5 and the injected flush error.
- **Complete write and flush, then dispatch:** preserves the boundary, but the preceding reply has
  already required a write. Collecting already-executed later replies recreates the same problem.
- **Stage mutations and commit after a shared write:** is not an IO-only optimization. It needs a
  separate authority, audit, quota, visibility, native-lock and acknowledgement design. This review
  neither implements nor authorizes that architecture or early replies.
- **GET-only specialization:** is not assumed pure merely because the command reads a value. Shared
  expiry, authorization and accounting need their own proof; it does not supply the requested SET
  mechanism or authorize omitting a harmed command from the comparison.

The narrow deduction is that coalescing **already-executed SET responses before delivery of the
previous response** conflicts with the retained frontier. For this design family, strict frontier
preservation requires a preceding write completion for each next mutation, so these readiness/
partial-progress variants do not supply the intended reduction in per-response writes. This is
not a theorem that every conceivable transport-specific optimization is impossible. A future
mechanism must demonstrate a different, semantically valid source of savings before D2.

## Decision and next boundary

No reviewed variant is admitted as a new product candidate. No new comparative benchmark,
dedicated-host run or qualification was started. The existing deep/shallow numerical policy is
retained verbatim in `w3-delivery-frontier-review.toml`; no favorable samples or weaker thresholds
are substituted for a semantic proof. Earlier kernel attribution remains useful owner evidence,
not authorization to retry a rejected implementation.

Any proposal that changes how many mutations may be visible before response completion, introduces
transactional staging or changes acknowledgement/flush guarantees needs explicit architectural
review outside this IO-only step. Without that review or a genuinely different same-semantics
mechanism, W3 remains rejected, C74 unresolved and the published 0.73 runtime unchanged.
