# Authority expected-name ownership attribution

Baseline is `a3ff05f45cc4bae012f9a5f5dff93d2741c7a59d`, including capacity-reserved
journal names. The single hypothesis is temporary owned expected names in
Directory::inventory. Preserve the observed BTreeSet, fresh directory scan,
UTF-8 conversion, duplicate/budget/parity guards, canonical order, error classes,
all other metadata/read checks and original 300/60/10 windows. Owned names for
opening and publication remain unchanged.

Compare each expected name with the same `{revision:016x}.{suffix}` formatter
writing to a 23-byte stack Cursor, not a String. Only commit/json suffixes and
bounded generations are involved; 16 hexadecimal digits plus dot plus the
six-byte suffix fit. Keep exact cardinality, including empty inventory and
extra observed entries. No new encoder, permissive parser, cache, container or
syscall optimization belongs in this candidate. Durable and API bytes stay equal.

Before runtime mutation freeze owned-reference and stack-proposal controls.
Differentially check depths 0..128, seeded mutations (seed 0x074e1a9e), all name
byte positions, missing/extra names, gaps, case, extensions, Unicode and NUL.
Check formatter capacity at hexadecimal boundaries and usize::MAX. Compare
actual filesystem inventory with the unchanged frozen scan/error classification.
Retain every historical assertion through a fail-closed exact source adapter;
reject unknown/duplicate helpers and hooks, preserve unrelated drift.

Measure a prebuilt observed set independently of filesystem I/O: empty,
healthy depths 1/7/128 and an invalid first-name depth-7 control. Two arms use
32 warmups and three alternating 1024-operation samples. There is no pretend
actual arm: real filesystem tests and unchanged eight-case full fixture inspect
own runtime acceptance. GNU 1.94 and MSRV 1.88 each get one Before and one
After invocation. Retain exact Before source/images/raw exits and observations
before implementation; never retry a failed measured invocation into healthy.

For depths one/seven, complete inspect must conserve twice the measured pure
comparison saving because it performs two inventories. All six non-authority
cases and all observed read/write and tracked kernel counts must stay equal.
Reject and retain a negative outcome, reverting only this candidate if either
correctness or conservation fails. Rust allocator requests are not CPU,
latency, copied bytes, retained memory, full ActiveAuthority or cache API gains.

Each phase requires focused tests, affected all-target check and strict
all-feature clippy. Use repository WSL rustfmt before checks. Before final
capture run historical/current contracts and format/docs/registry/local
performance gates. Freeze non-overwrite recipes before capture; compare JSON
structurally, preserving fields, types and array order. Rehash all thirteen
preceding packets without editing/resealing; workload, reference and
qualification inputs stay identical. Seal the new packet only after review.

Local nonroot synthetic controls only. No server, install, supervisor, clock
or service changes, new live matrix, product workload, qualification, or push
through the red native gate. Original delivery, writer cessation, lifecycle,
SIGKILL/lost-ACK, reconnect and trust/enrollment gates remain independent and
closed. A request reduction or different boot is not evidence of time repair.

## Local outcome

Before source `fe387e897e5ab56397187088d8fdcdc7396c4842` and After/runtime
`be357d1073a6dc0cfccfc50437dc253311e1d847` each have one capture per compiler.
The [paired packet](local-runs/authority-expected-name-20261021/README.md)
confirms zero heap requests for the pure stack comparison at depths 1/7/128,
versus 2/14/256 allocations and 44/308/5632 requested bytes in the reference.
All three samples on both compilers agree. Empty and invalid first-name
controls preserve their decisions; reallocations remain zero.

Complete fixture inspect conserves twice the isolated reduction: 29/0/1806
becomes 25/0/1718 at one generation, and 129/0/9116 becomes 101/0/8500 at seven,
in allocations/reallocations/requested bytes per operation. Reads stay 4/28;
the six other cases and tracked kernel observations stay equal. Frozen
reference/proposal/workload/qualification inputs remain unchanged.

Both lanes pass 196 authority tests, ancillary checks, all-target check and
strict all-feature lint. All 375 source assertions and repository checks pass.
The first new adapter refuses a known older restored inventory template;
only that explicit historical input is added, retaining the refusal and every
old assertion. No measured invocation is replaced. The candidate is accepted
for local Rust request reduction, not CPU/latency, full ActiveAuthority,
cache API, retained-memory or native/server admission. Before/After boots
differ; all old live refusals and independent gates remain closed.

Evidence `bdf4cadabfeb9695f94db8d89bfd20c69d79bb00` seals 95 members plus
inventory SHA256
`4249e3cd970a5159951a22b5e40762b4aaa97eee003118d664bb67110376464f`.
All thirteen prior packets remain intact, eight structural JSON and ten scope
mutations are refused, and all 96 Git entries preserve raw bytes. The sealed
packet is immutable. Remaining observed-name/record-read costs need their own
attribution baseline; no cached live observation or timing waiver is implied.
