# Signed held placement requirements

Inherits `docs/RULES.md` and step four of the approved
[attempt root plan](cgroup-root-enrollment-plan-20261019.md). Baseline is
`b23a503b`. Keep all 423 previous supervisor sources, the original complete
validation graph, 46 effect brackets and absolute 300/60/10 origins unchanged.
No server, installation, supervisor operation, original/system-bus send,
product workload or new fault campaign is part of this package.

The original manager message still uses the v1 worker specification, whose
Slice is system.slice. Root guardians retain another attempt root. Delivering
that old message does not place its process within the root and would also
execute the worker before the required held-launcher barrier. Neither the
successful creator fault cohort nor ReplyState preparation closes that gap.

Implement only the signed placement part of the separate held projection.
Use a new v3 schema/domain; keep old v1/v2 signatures and requirements byte-for-byte.
Bind the complete validated nested v2 launch claims and exact 104-byte v2 Enroll
packet, including original-context digest, root identity and absolute bound.
Require the packet's attempt digest to match the same original identity using
the existing hc074-original-root-attempt-v1 tuple. Reject an Armed/refusal packet
or an unrelated attempt, not merely malformed bytes.

Derive expected names rather than accepting paths: system-hc074 plus the full
attempt digest names a slice directly beneath system.slice; the existing fixed
cell-one unit name names its expected child. All name components are fixed text
or validated lowercase hashes. This is a local expected-placement candidate,
not a live slice, permitted population or a TransientUnitSpec. Root creation,
actual systemd placement/ownership/inode and writer confinement remain separate.
Do not invent a launcher installation path or substitute its future ExecStart.

Sign canonical bounded newline-terminated JSON. Retain the existing 16 KiB body
and 32 KiB envelope limits. Reject unknown/duplicate fields, future schemas,
noncanonical bytes, wrong signature domain/key, changed claims/digests, invalid
hex and inconsistent root/attempt. The supplied-key inspector returns claims
only; it cannot enroll trust, create a root, access a clock/FD, arm, send,
release a barrier, complete or unlock. Add compile-fail guards for that boundary.
Register the additive wire scope in COMPAT without upgrading old signatures.

First retain ordinary GNU/MSRV baseline results, then a missing-module red
compile. Cover valid canonical roundtrip, exact projection and safe deterministic
names, seed-driven identities, all signed-field mutations, all root packet kinds,
zero/oversized/foreign roots, unknown/duplicate/budget/noncanonical documents,
cross-version/domain and wrong-key refusal. Keep every old test body unchanged.
An exact additive source adapter restores all 423 original sources, COMPAT and
registry; missing/duplicate hooks or unrelated drift must fail. Source-contract
checks may use four libtest threads after confirming no process-global mutations;
this is not a timed attribution or a change to any live/fault schedule.

Run portable and GNU/MSRV units/doc checks, strict all-target/all-feature lint,
formatting, source contracts, registry, docs and the local non-promotable
performance contract. Retain source/image/recipe digests and a new byte-exact
packet. Keep the boot-comparison packet and qualification manifest unchanged.
The next implementation is the original-owner trust/artifact join and complete
held-unit projection, not sending this expected placement as an ordinary worker.
