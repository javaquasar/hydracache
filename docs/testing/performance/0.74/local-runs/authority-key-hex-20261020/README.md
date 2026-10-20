# Authority issuer key formatting allocation control

This local control tests one change: use the existing exact-capacity hex encoder
instead of per-byte formatting in authority `key_hex`. Canonical key bytes,
binding order, fresh journal/metadata observations and 300/60/10 live deadlines
remain unchanged. No server, clock, service or product operation is in scope.

The [preregistered plan](../../authority-key-hex-plan-20261020.md) fixes three
encoder arms: frozen old reference, proposed `crate::hex`, and actual runtime
`key_hex`. Before and After each contain three 1024-operation samples per arm,
32 warmups and rotating arm order. The allocator counts successful Rust requests
on the current thread, not libc calls, retained memory, CPU or latency.

Each stage also runs the unchanged eight-case leaf/complete Origin/fixture
authority control. Its allocation, I/O and kernel-count observations are separate.
The authority fixtures contain one or seven generations; fixed production
ancestry and the full ActiveAuthority wrapper are not measured. Setup and
durable journal publication are outside capture. This is not a product benchmark.

The after-stage encoder reduction must equal the complete inspect reduction at
both journal depths. All other cases and read/write/kernel observation counts
must stay equal. This conservation check distinguishes an isolated formatting
change from an accidental reduction in required work. Requested bytes include
realloc sizes; no syscall or full-frame latency gain is inferred from them.

`check.ps1` records native exits and hashes each log without overwriting.
`profile.ps1` binds the clean committed source, compiler image before/after hash,
recipe and normalized committed input bytes. The two probes run serially.
Images are rebuildable from source/toolchain identities, not copied into Git.
`review.ps1` recomputes both-stage results and conservation, checks receipts,
rehashes eight prior sealed packets and rejects summary scope mutations.
Once sealed, this packet must not be edited or resealed.

The earlier live clock-order refusals are retained independently. A successful
local allocation candidate cannot admit original execution, authorize a server
rehearsal or waive lifecycle, trust, writer retirement and reconnect gates.

## Results

Before source `05093a506034550cf44454a5a27ad0414cc9a413` is retained before
candidate `33ddeff958a1c6ee3712def385913bcf861b55f4`. Three samples per stage
on each compiler give identical per-operation request counts:

| Operation | Allocations Before to After | Reallocations Before to After | Requested bytes Before to After | Read requests Before to After |
| --- | ---: | ---: | ---: | ---: |
| Issuer key formatting | 32 to 1 | 3 to 0 | 368 to 64 | Not observed in leaf capture |
| Complete fixture inspect with 1 generation | 68 to 37 | 11 to 8 | 3014 to 2710 | 4 to 4 |
| Complete fixture inspect with 7 generations | 184 to 153 | 59 to 56 | 13292 to 12988 | 28 to 28 |

The full inspect reduction equals the isolated leaf reduction in every sample:
31 allocations, 3 reallocations and 304 requested bytes. All six non-authority
cases and kernel/read/write counts remain equal; each measured batch reports
zero write requests. The proposed and frozen reference arms remain unchanged
across stages. There is no borrowed validation result or skipped observation.

Each compiler passes the 187-test authority suite before/after, leaving ten
manual/privileged tests ignored, plus the seeded equivalence and other focused
kernel/trust suites. Check/strict clippy pass on both lanes. All 365 historical
and current source/contract assertions and format/docs/registry/local contracts
pass. The substitution is locally accepted for request reduction only.

Before boot is `415453e6-011c-465c-a174-119a42f82903`; After boot is
`85520221-889b-4750-9659-10029b8994a4`. These separate processes/boots are not
one continuous native campaign. No clock/service controls occur, no native
matrix is retried, and no time/latency/CPU or server admission claim follows.
Qualification manifest bytes remain unchanged. Remaining temporary context,
name and inventory ownership requires separate attribution and candidates.
