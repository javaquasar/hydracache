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
