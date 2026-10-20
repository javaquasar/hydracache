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
