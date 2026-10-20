# Expected inventory attribution

Baseline is `85cec4a666e777c57a6cf05bfa4b4ec2641c9b39`. The single
hypothesis is materialization of the expected `BTreeSet` in fixture
`Directory::inventory`. Keep the observed set, fresh directory scan, UTF-8
conversion, duplicate/budget guard, parity check and every live validation
unchanged. Compare its sorted names with a generated sequence ordered by
revision, then commit/json. Keep the exact existing name formatter; no caching,
name-encoding optimization, removed observation or changed error precedence.

Before runtime mutation freeze a complete reference inventory, a proposed
suffix comparison and the actual implementation. Differential tests cover
every bounded healthy depth, malformed names, gaps, wrong extensions, invalid
UTF-8, odd and over-budget inventories, directory errors, shuffled names and
seeded mutations. Exact source guards preserve the original prefix and refuse
unknown implementations. Test hooks stay after runtime declarations.

Measure only suffix comparison using prepared observed sets (setup outside
capture): empty, healthy 1/7/128, and same-cardinality invalid names. Use 32
warmups, three alternated two-arm 1024-operation samples, GNU 1.94 and MSRV 1.88.
The isolated arms are frozen reference/proposal; actual runtime is tested by
complete filesystem inventory and unchanged full-inspect measurements. Retain
Before source, images, recipes, exits and observations before mutation. Repeat
once after one isolated runtime change. A failed measurement is retained, not
retried into a healthy cohort. Early live failures must still skip this suffix.

Acceptance requires unchanged complete eight-case inspect/Origin controls.
At one/seven generations, complete fixture inspect must save exactly twice
the corresponding isolated comparison reduction: inventory runs twice per
inspect. Six non-authority cases, all read/write requests and fresh kernel
observations must remain equal. Do not infer CPU, latency, retained memory,
all syscalls, full ActiveAuthority cost or cache API performance from successful
Rust allocation/reallocation requests and requested bytes.

Each phase requires focused tests, affected-package all-target check and strict
all-feature clippy. Before final capture run historical/current contracts and
format/docs/registry/local performance checks. Rehash eleven preceding sealed
packets without altering them. If correctness or full-inspect conservation
fails, preserve the negative result and revert only this runtime candidate.

Local nonroot synthetic controls only: no new server run, installation,
supervisor operation, clock/service changes, product workload or qualification.
All live predicates and original 300/60/10 windows remain. Old clock-order
refusals, execution/trust/lifecycle/reconnect/SIGKILL/lost-ACK gates stay closed;
an allocation result does not transfer admission to a rebuilt tool. No push
through the existing red live gate.
