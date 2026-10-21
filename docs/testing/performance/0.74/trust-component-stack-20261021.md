# Short owned trust component names

## Decision and implementation boundary

Keep this isolated allocation candidate after the
[preregistered experiment](trust-component-stack-plan-20261021.md), against
baseline `a72457aade616e67d2fc5693f6105c8063d6e9e1`. GNU 1.94 and MSRV 1.88
pure controls support removal of short-name requests. Full preparation timing
and server readiness are not measured by this work.

Candidate source: `a7123c37985bd129b10eab969ab260a1fc551745`;
preregistration: `13aaa0b5`.

The shared worker/controller/held-runtime/OIDC `child` helper now constructs a
private owned name. Names shorter than 32 bytes use a zero-terminated local
32-byte array; all other inputs retain CString conversion. Short embedded-NUL
names return the same Invalid before open without allocating the NulError
payload that the helper previously discarded. Long embedded-NUL inputs retain
the old fallback. Empty, Unicode, slash/dot and arbitrary-length compatibility
remain: this conversion does not introduce a path/component validator.

Only the constructor, private module/test hooks and test-only CString import
change in the old trust file. The original parent, pointer to identical
terminated bytes, flags, one openat, negative-fd handling, last-error and File
construction stay in their original order. No unsafe, observation, caching,
shared state, public representation or new authority is added. Both fresh-check
counts/order and the original 300/60/10-second windows remain. The 32-byte
array is not a claim about the complete local type or stack footprint.

## Allocation comparison

The actual helper is measured through a pure adapter with the same 21 valid
and four NUL shapes and 256 constructions each as the frozen baseline. The
unchanged baseline is rerun and agrees with its earlier sealed counters. Both
compilers report the same results:

| Corpus per compiler | Reference requests / bytes | Candidate requests / bytes |
| --- | ---: | ---: |
| All valid shapes, 5376 constructions | 5376 / 1274880 | 1536 / 1221632 |
| Six fixed names, 1536 constructions included above | 1536 / 29696 | 0 / 0 |
| Four short NUL shapes, 1024 separate constructions | 1024 / 5888 | 0 / 0 |

No reallocation occurs. The six long valid shapes account for all remaining
candidate requests and retain exactly their reference bytes. Explicit long-NUL
controls also retain one length+1 request per conversion. These are Rust
allocator requests, not allocation usable size, CPU, RSS, retained memory or
full preparation wall time. Initialization, copying, branching and local storage
still have costs. The mixed corpus is not a production frequency distribution;
neither its aggregate nor old WorkerSource spans estimate admission headroom.

## Semantic and source controls

Five new pure controls cover valid/NUL requests, a 512-case complete-body
differential with baseline seed `0x074c57a1`, explicit first-error/long-NUL
fallback cases and independent ownership. The adapter verifies that the actual
production pointer equals the pointer to the exact terminated bytes. Input
mutation, moving the name and nested independent names preserve those bytes.

The complete child model substitutes only openat, last-error and File
construction. Restoring those operands yields the actual production body;
reversing the name/view expressions yields the complete frozen baseline model.
The comparison preserves parent/flags, zero-fd success, negative-fd last-error,
NUL priority and no EINTR retry. The corpus and frozen reference body are
token-compared with the unchanged earlier controls. These synthetic operands
do not prove actual filesystem races, descriptor ownership transfer or writer
cessation.

Whole-file guards restore both changed old supervisor files with exact
constructor/import/hook reversals. Historical canaries require the entire
earlier trust source before accepting this successor, rather than exempting
its filename. Mutation controls reject changed flags, pointer, negative-fd
predicate or last-error behavior. The private helper and pure adapters have
no live capabilities or new unsafe code.

## First refusal and focused checks

Both first Linux compiles refuse two old tests that inherited the removed
CString import. Initial source/guard and logs remain separate. The corrected
cohort restores that import under `cfg(test)` and adjusts its exact guard
reversal; the name algorithm and all runtime predicates remain unchanged.
This compile repair does not repeat or replace a live refusal.

The corrected cohort passes 31 source canaries, 21 pure controls per Linux
compiler, affected all-target check and strict all-feature clippy on host/GNU/
MSRV, workspace formatting, doc-check, the unchanged 186-gate registry and
local non-promotable performance contract. This is a focused step gate, not
full milestone or ship verification.

The [packet](local-runs/trust-component-stack-20261021/summary.json) keeps both
cohorts and source/method snapshots. Its
[offline reviewer](../../../../tools/trust-component-stack-074/review.ps1)
derives counters and verifies eight earlier raw seals and all 348 members,
without editing their frozen source-pinned reviewers or qualification bytes.

The packet has 53 members plus its inventory; raw inventory SHA-256:
`e588fd8ae6b06a6e808446328b819240c4bb0e35955309e843ddb1078668fd85`.

## Remaining boundary and next leaf

Both first Causal(Time) ages remain 10042390247/10010835739 ns. No original-owner
or live profile, clock/service change, server/SSH/Actions, installation,
supervisor or product workload is run. Successful full material revalidation,
cleanup/EOF/fence, authenticated delivery/all-writer cessation, SIGKILL/lost-ACK/
reconnect and server gates stay independent and closed. A future timing
comparison needs separate admission and must retain those first refusals.

Next review collector peer-vector growth as a separate baseline and error/
canonical-union contract. Do not combine it with this candidate or claim
current full-path gains from old trace counts.
