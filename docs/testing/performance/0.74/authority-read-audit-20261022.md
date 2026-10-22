# Fresh authority read ownership boundaries

## Decision

Keep retained Leaf.raw ownership unchanged. The next isolated hypothesis is
temporary storage for fresh revalidation bytes, not removing the fresh read or
reusing expected bytes as evidence. A separate bounded-read reference baseline
is required before implementing that hypothesis. This offline audit introduces
neither an allocation candidate nor a performance result.

The [preregistered scope](authority-read-audit-plan-20261022.md) starts at
`3f48a7c9`. The read-only [auditor and mutation controls](../../../../tools/offline-authority-read-audit-074/README.md)
pin complete authority fixture, registry, fixed backend and traced-owner source.
Sixteen older packet seals and 928 members, including the rejected boot-inline
experiment, are reconstructed without executing their probes.
Scope commit: `8fb9f314`; auditor/check recipe:
`338bfd6b0b63b6c4e2747b8ae22ccf8fc8e7f6fc`.
Evidence commit: `a71234e3`. Its inventory seals 27 members with SHA-256
`1c5176e4e53eb56d21e66c061a9b580529b4930d32682e4397db4b5d3e0f6591`.

## Two byte owners with different lifetimes

Leaf::open stores a File, stamp, owned raw bytes and name. The raw bytes are the
expected content for later comparisons; they also participate in marker/history
verification. Their lifetime extends with the retained Leaf. Replacing them
with a borrowed view or overwriting them during a fresh read would change the
ownership and comparison contract.

Leaf::revalidate first checks the retained File's stamp. It then calls Leaf::read
to open the named object independently and obtains a second File/stamp/Vec
tuple. That fresh File remains alive through both stamp and byte comparisons.
The second Vec is temporary; its shorter lifetime is the only next storage
hypothesis selected here. Dropping tuple field zero before comparison or using
a cloned original File instead of reopening the name does not preserve the
current observation bracket.

## Complete fresh read and refusal bracket

The current shared backend performs these operations in order:

1. Open the name relative to the retained directory, read-only, no-follow,
   close-on-exec and nonblocking.
2. Read before metadata; require a regular file, exact UID/GID and mode 0400,
   with exactly one link.
3. Compare device with fresh parent metadata and check the before length against
   the caller's fixed budget.
4. Capture all eleven stamp fields and construct a Vec with before-length capacity.
5. Read through take(budget + 1) and read_to_end, propagating errors.
6. Read after metadata, compare the whole stamp, then require the exact original
   byte length. Return owned File/stamp/bytes together.

Short-circuit order matters: a changed after-stamp refuses before the raw-length
comparison. The fixed budgets are 2048 bytes for a record, 65 for a marker and
128 generations. The extra read byte is not a larger accepted record size;
post-read metadata and exact-length predicates still apply. Reading exactly the
old length is not an equivalent EOF/growth observation.

Low-level Leaf::read does not itself latch registry refusal. The shared registry
brackets record/marker revalidations with two directory and two inventory checks,
then makes any error sticky. FixedAuthorityRegistry::inspect delegates to that
shared inspect path, which revalidates before binding independently supplied
trust/host inputs. The audit keeps those caller responsibilities separate from
the buffer expression.

## Why the constructor is not a total allocation measurement

There is one explicit Vec::with_capacity expression in the read body. That is
a source fact, not evidence for one allocation, a particular requested capacity,
or an allocation-free read. Read::take/read_to_end can involve spare capacity,
growth, EOF probing and error/partial-read behavior. Returned ownership and drop
also differ between initial Leaf::open and temporary Leaf::revalidate reads.

The next baseline must separate construction, bounded reading, returning and
dropping ownership, and the complete bracket. It must preserve the original
File lifetime, metadata schedule, first-error propagation and caller latching.
Cover empty/short/exact-bound/over-bound data, fragmented reads, Interrupted and
other errors, metadata/content drift and reused-buffer tail handling. Pure
generic Read controls alone cannot establish GNU/Linux File allocation or
syscall cost; their scope must stay explicit.

No fixed-size scratch, reusable buffer, cached observation, shared expected
bytes or read_exact replacement is adopted here. Actual retained/stack footprint
and complete-path cost remain acceptance inputs for any later candidate.

## Verification and remaining readiness

Eleven new offline tests exercise ownership classification, exact expression
order/counts, whole-source pins, metadata fields, bounds/EOF/error shortcuts,
fresh-descriptor lifetime, caller latching, capability restrictions and exclusive
output publication. During development, a mutation exposed a field guard that
also matched the stamp constructor; it now checks the declaration separately.
That test-guard repair did not change authority sources or an old failure.

The focused closing gate also runs the preceding thirty-six offline controls,
forty-six Windows source canaries, xtask check/strict clippy/format and the docs,
registry and local non-promotable performance contracts. The existing traced
leaf differential is source-audited, not rerun as a kernel test. The retained
[facts](local-runs/authority-read-audit-20261022/facts.json) preserve missing cost
counters as null rather than manufacturing a baseline from source appearance.

Runtime, qualification digest, original 300/60/10 windows and first Causal(Time)
ages 10042390247/10010835739 ns remain unchanged. Original material/cleanup,
authenticated delivery/all writers, full SIGKILL/lost-ACK, reconnect and server
gates remain closed. No new measurement, WSL, clock/service change, installation,
supervisor operation or server/product workload occurs in this audit.
