# Authority name capacity attribution

Baseline is `36f9f61ff0989340cefae2107f726b48c650c7a8`, including the accepted
expected-inventory stream. The single hypothesis is String capacity growth
inside private `name(revision, suffix)`. All runtime call sites use json or
commit suffixes; inventory calls it twice per generation and runs twice per
complete inspect. Do not alter the observed set, scan, name bytes, formatter,
metadata/read checks, failure order or original 300/60/10 windows.

Reserve 17 plus suffix byte length, then write the same 16-digit hexadecimal
revision, dot and suffix with the same formatter. On supported usize widths the
revision takes exactly 16 padded digits, including usize::MAX. String's fmt
writer cannot return an error. No stack encoder, parser, cache, container or
allocation-policy change belongs in this candidate. Durable bytes and API stay
unchanged. This private helper is not a production execution or clock gate.

Before runtime mutation freeze reference/proposal/actual controls. Compare
every bounded revision, hexadecimal boundaries, usize::MAX and seeded revisions
with empty, normal, long, NUL and Unicode suffixes. Exercise journal canonical
bytes, read-back, mutation refusals and original metadata through the existing
authority suite. Preserve all old assertions: an exact source-input adapter may
restore only known helper/layout additions, and must reject unknown bodies,
hooks and changes outside the helper. Test hooks stay at the fixture tail.

Measure json and commit names at revisions 1, 7 and 128, plus a long suffix
control. Each compiler (GNU 1.94, MSRV 1.88) uses one Before and one After
invocation, 32 warmups and three rotated 1024-operation samples. Check actual
against the frozen reference Before and proposal After. Retain Before source,
images, native exits and raw observations before mutation. Never replace or
retry a failed measured invocation into a healthy cohort.

The unchanged complete eight-case inspect control owns acceptance. At depths
one/seven it must conserve twice the sum of record and commit name reductions
at the corresponding revision, multiplied by depth. Revisions within each
bounded depth must have equal costs. All read/write and tracked kernel counts,
and all six non-authority cases, stay equal. Reject/retain a negative result
and revert only the candidate if correctness or conservation fails. Allocator
counts mean successful Rust requests, not CPU, latency, copied bytes, retained
memory, full ActiveAuthority or cache API performance.

Each phase requires targeted tests, affected-package all-target checks and
strict all-feature clippy. Use repository rustfmt before check/capture, not
standalone snippet defaults. Before final measurement run all historical/current
contracts and format/docs/registry/local performance gates. Freeze non-overwrite
recipes with collision-free step names; compare JSON structurally, retaining
array order, types, fields and values. Rehash twelve prior packets, never reseal
or modify them. Qualification/workload/reference inputs stay identical.

Local nonroot synthetic controls only. No server, installation, supervisor,
clock/service change, new live matrix, product workload, qualification or push
through the existing red native gate. Original delivery, writer cessation,
lifecycle, SIGKILL/lost-ACK, reconnect and trust/enrollment gates stay closed.
Different process/boot observations cannot establish continuity or repair time.

## Local outcome

Before source `5364b8215c3892f40987d3284dff6be853208e4d` and After/runtime
`433a7cb1806d7296c8dff72bf83620eb48a91d42` each have one capture per compiler,
with three samples and unchanged workload/reference/qualification inputs. The
[paired packet](local-runs/authority-name-capacity-20261021/README.md) confirms
two fewer reallocations per json/commit name, with unchanged initial allocation
count and exact bytes. Full fixture inspect conserves 8 reallocations/136
requested bytes saved at one generation, 56/952 at seven. Full counts change
29/8/1942 to 29/0/1806 and 129/56/10068 to 129/0/9116 respectively. Reads stay
4/28; six other cases and tracked kernel counts stay equal.

Both lanes pass 194 authority tests, ancillary controls, all-target check and
strict all-feature lint. All 373 historical/current assertions and repository
checks pass. A new negative canary initially targets Entry::open's revision,
not name(); only its selector changes before measurement. The refusal remains
documented, every old assertion is preserved, and no capture is retried.

The capacity candidate is locally accepted for Rust request reduction only.
Initial name allocation remains; CPU/latency, retained memory, full
ActiveAuthority and cache API gains are not claimed. Before/After boots differ;
all old live-clock and independent execution/trust/lifecycle gates remain
closed. No server, new live matrix, clock/service or product operation occurs.

Evidence `8d1be42bf5ccded5f53ae1b687a8993d4c63667a` seals 92 members plus
inventory SHA256
`2d94764f08a89ed50e4e1d8ce47906382fb6be6c796bd585e7d0fbdc5f628362`.
All twelve preceding packets remain intact; ten scope mutations and eight
structural JSON negative controls pass. This packet is immutable; a further
expected-name ownership hypothesis needs its own baseline and capture.
