# Signed held role provenance binding checks

Baseline is 697a911aae11cab00b67046c80b42713d3d9e3e7. Implementation b479d5a0
adds the separate v4 domain/grammar and static inspector. Selected acceptance
source is c24f460abcadef8a1d916aa1c5ea53a7b14c26fb. Inherits docs/RULES.md and
the [binding plan](../../held-role-binding-plan-20261020.md).

## Contract and scope

The complete signed v3 placement is nested in a newly signed v4 body with the
exact held-policy digest, canonical signed receipt digest and matching epoch.
Both controller signatures use the independently checked policy's controller;
the receipt keeps its separate held-builder signature. Policy/receipt/consent
epochs, repository and all role image expectations must agree. The returned
opaque Inspection only compares exact placement bytes and checks supplied
content. Another signed attempt is valid static data, but not the retained
original placement. No incoming document selects its own trusted verifier.

All policies, keys, ELF headers and build observations here are invented test
fixtures. No protected producer emits a real role receipt. No live original
policy/epoch/frame join, FD/process/cgroup proof, replay slot, key enrollment,
send/start, SIGKILL/lost-ACK matrix, server/supervisor or product work occurs.
The active authority registry and reviewed held-key/policy admission remain
separate; v4 signed claims cannot replace those original owners or clocks.

## Baseline and negative controls

windows-baseline.log passes the sixteen old static controls. The initial new
test then refuses to compile for the absent held::role_binding module; retain
missing-module-red.log. first-control.log passes only the first new control,
expanded-windows.log runs all twelve new controls, and GNU/MSRV precommit logs
run the complete 38-test selection. These are development checks, not accepted
source/image receipts. Precommit strict lint and fourteen source guards are
retained separately.

Twelve new portable controls cover canonical/unknown/duplicate/nested fields,
signature encoding and domains, wrong keys at either signature, valid-but-other
receipts/policies, re-signed wrong epochs/repository/digests and each role, exact
placement/content mismatch and all size bounds. A new valid lease/attempt cannot
match the old placement. Sixty-four bit corruptions use seed 0x0074_b14d. The
external signing-message helper also refuses invalid bindings before returning
bytes; tests deliberately bypass it to check re-signed malformed semantics.

## Selected source gate

checks.ps1 requires unchanged selected HEAD and clean runtime/tests/xtask/lock/
COMPAT/registry/qualification inputs before and after each cohort. It records
compiler, recipe and actual test image hashes before execution and compares image
hashes afterward. It refuses compiler panic text despite a successful Cargo exit.
Successful commands with empty stdout produce explicit native-exit receipts,
instead of assuming that Tee-Object created a log. This closes the preceding
package's wrapper issue without altering its sealed packet.

Windows runs two complete 28-test rounds (sixteen old plus twelve v4) and four
capability compile-fail examples. GNU 1.94 and MSRV 1.88 each run two complete
38-test rounds (also ten unchanged Linux role-file controls) and six capability
examples. The new examples require E0599 for start and E0616 for private build
extraction; unrelated compiler errors are not that proof. Strict lint runs on
all three platforms/toolchains.

Fourteen source/adapter tests reconstruct all 441 preceding runtime files after
removing one exact module hook and permitting one named new source; the new
COMPAT block is removed exactly, never an old schema. Three predecessor bridge
controls, scoped fmt, registry/docs and local non-promotable contract are separate
gates. The full 321 suite, original live matrix and launcher process cuts are not
rerun by this static document package. Existing 46 brackets and 300/60/10 origins
remain byte-exact. Qualification manifest SHA256 remains
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

verify.ps1 checks exact membership/hashes, source/recipe/image metadata, test
counts, seed, silent format receipt and false scope flags. Thirty-four in-memory
receipt/format metadata mutations must refuse. The sealed inventory is local
evidence, not production provenance or server admission. The predecessor role-file
packet verifies unchanged; no negative log is overwritten or promoted.

Next implement independently admitted held trust and the consuming original
FixedPlacement/role-file join using original controller and live epoch around
the unchanged complete frame. Attribute hashing on actual role images before
that deadline-sensitive join; complete all-writer retirement, durable lifecycle
and reconnect before separately authorized server rehearsal.
