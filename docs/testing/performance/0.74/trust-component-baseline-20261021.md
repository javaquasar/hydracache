# Pure baseline for trust component C strings

## Frozen operation and scope

This [preregistered baseline](trust-component-baseline-plan-20261021.md) freezes
`CString::new(name)` in the private trust-reader `child` helper before any
replacement. Worker, controller, held-runtime and OIDC readers share that
helper. Six fixed component names are tested alongside generic inputs; they
are not six calls per validation or a global multiplier for the old trace.

Baseline source: `a72457aade616e67d2fc5693f6105c8063d6e9e1`;
preregistration: `2320066d`.

The production trust readers remain unchanged. Only a private opt-in module
hook is added to the existing allocation-test binary. Source canaries reverse
that hook and compare the complete previous source; the earlier guard chain
uses the same exact reversal, not a filename exemption. No wire/durable type,
policy, observation, call order, deadline or qualification identity changes.

## Measured allocation baseline

GNU 1.94 and MSRV 1.88 report one Rust request of length+1 bytes per conversion,
with no reallocation, on the same corpus and 256 constructions per named case:

| Corpus per compiler | Constructions and allocations | Requested bytes |
| --- | ---: | ---: |
| All 21 valid shapes | 5376 | 1274880 |
| Six fixed component names, included above | 1536 | 29696 |
| Four embedded-NUL shapes, separate | 1024 | 5888 |

The fixed byte lengths are 3 (`etc`), 15 (`hydracache-perf`), 21 (worker),
25 (controller), 27 (held-runtime) and 19 (OIDC). Generic controls include
0/1/30/31/32/63/64/255/256/4096-byte names, Unicode, parent/dot and absolute
forms. CString conversion accepts these NUL-free shapes; it does not prove
that they are safe components or that an OS open would accept them.

Embedded-NUL conversion still allocates before returning its owned error
payload. Leading/interior/trailing and Unicode-NUL controls retain the exact
original bytes and first NUL byte position. No syscall follows this conversion
refusal. These values describe the current standard-library operation, not
current full preparation latency, CPU, RSS, retained memory or a speedup.

## Complete child-body contract

The pure model freezes the complete production body, substituting only the
openat operand, last-error operand and File construction. Source token checks
restore those three expressions and compare every other token. Mutation
controls reject changed names, flags or negative-descriptor handling.

Five pure tests measure valid and NUL cases, preserve independent owned values,
check explicit first errors/flags/generic names, and exercise a 512-case schedule
with seed `0x074c57a1`. They verify terminated bytes, original parent and exact
O_RDONLY/O_NOFOLLOW/O_CLOEXEC/O_NONBLOCK/conditional O_DIRECTORY flags, one open
callback, last-error only after a negative result, NUL priority and no EINTR
retry. Zero is accepted by the model as a successful descriptor result. There
is no real descriptor, File ownership transfer, filesystem race or open here.

## Checks and retained first refusal

Both first Linux compiles refuse the new test's nonexistent
`NulError::as_bytes` method. The first source and logs remain. One test-only
repair consumes the original error via `into_vec().as_slice()`; it does not
clone the payload, alter the production reader or repeat a live refusal.

The distinct corrected cohort passes 28 source canaries and affected all-target
check/strict all-feature clippy on host. Each Linux compiler passes five new
pure controls, four earlier stack controls, five unchanged document baseline
controls and two counter controls, then affected check/strict lint. Workspace
format, doc-check, unchanged 186-gate registry and local non-promotable
performance contract pass. This is a focused step gate, not a ship gate.

The [sealed packet](local-runs/trust-component-baseline-20261021/summary.json)
retains both cohorts, source/method snapshots and structural logs. Its
[offline reviewer](../../../../tools/trust-component-baseline-074/review.ps1)
also verifies all seven earlier raw seals and their 303 members without changing
the historical source-pinned reviewers. Initial errors are not replaced by
corrected results. Original windows remain 300/60/10 seconds and first
Causal(Time) ages remain 10042390247/10010835739 ns.

The packet contains 45 members plus its inventory; raw inventory SHA-256:
`7eacd69041a722fc3d6153ef8be43eff87bf945a8fb6ccdd456acadff1bfcc9d`.

## Next isolated candidate

A separate short owned C-string representation may remove requests for the
fixed names while retaining arbitrary-length CString fallback. It must preserve
embedded-NUL refusal before open, exact bytes/flags/parent/error order and
independent lifetimes. This baseline implements no such runtime candidate and
does not time one. Old WorkerSource or ReservedLive spans cannot be attributed
to these small requests or extrapolated into post-candidate savings.

No original-owner/live profile, clock/service change, server/SSH/Actions,
installation, supervisor or product workload is run. Full material revalidation,
cleanup/EOF/fence, authenticated delivery/all-writer cessation, SIGKILL/lost-ACK/
reconnect and server gates remain independent and closed.
