# Local held launcher process and descriptor checks

Baseline 2a86c1fdeb37a767ea2998d96f9a1343afe952e6 is identified by the retained
baseline-source.log; selected implementation is
68b71834b7f9ec13a48ccbea93cd244935e9cf0b. The first complete cohort is separately
retained at 25888b65b3edad116d891b56ce5f61b69b510d76. Inherits docs/RULES.md and the
[local helper plan](../../held-launcher-plan-20261020.md).

The new executable is real local inherited-FD mechanics. All executed worker
images in the matrix are a tiny synthetic syscall-only ELF, not HydraCache.
The root-owned wrong-hash case reads /usr/bin/true but refuses before execution.
Debug helper/test binaries are not a signed production role bundle.

## Baseline and preliminary checks

The preceding build-authority packet verifies, including its 26 optional metadata
mutation controls. Sixteen build-authority tests pass before implementation; a
missing-target compiler refusal follows. Formatter parsing and two integer-type
inference errors are retained before their fixes. A deliberately unregistered
integration target refuses for its missing cfg/ignored gate entries, then both
are registered. No runtime expectation or deadline was relaxed to pass these.

The first --ignored development selection runs the process matrix but filters
all three pure tests; its zero-test pure result is not their acceptance gate.
The separate corrected pure selection and final selected-source runs execute
all three tests. Earlier 20/22-case process logs are preliminary; the final
matrix initially has 26 named cases, with seed 0x0074_e7ec. The following audit
adds O_PATH stdin: it must refuse before Ready, but initially waits for Release.
opath-regression-red.log retains that failed expectation and its owned cleanup.
The one-line guard adds O_PATH to the access-mode mask. The first log named
opath-regression-green.log is instead a compiler refusal in an added argv oracle
(the workspace SHA type lacks LowerHex), not a passing regression. Portable
per-byte hex fixes that harness error; the separate corrected green log passes
all 27 cases. The intermediate fix commit is not treated as an accepted gate.

## Selected source and image checks

final/checks.ps1 requires unchanged selected HEAD and clean runtime/test/xtask/lock/
COMPAT/registry/qualification inputs before and after every cohort. Recipe,
compiler, actual helper and test executable SHA256 values are recorded before
execution and compared afterward. The embedded Cargo helper path is selected
from the same build's JSON, not discovered by directory enumeration.

GNU 1.94 and MSRV 1.88 each pass three pure protocol tests, two rounds of all
27 process cases, sixteen unchanged build-authority integration tests and two
capability compile-fail examples. Final source/image/recipe receipts and logs are
in final/; earlier root-level 26-case receipts remain preliminary. Strict lint
passes on both. Windows builds the
real unsupported-platform fallback, verifies exit 2, and passes the sixteen/two
build-authority controls. See final/repository-receipt.json for the ten source/adapter
guards, three predecessor bridge controls, Windows strict lint, scoped fmt,
registry/docs and local non-promotable contract gate.

Healthy execution proves no marker before release, one marker afterward, same
PID/start ticks, changed executable identity and only descriptors 0/1/2/5 after
exec, with fully checked argv and empty environment. Final logs retain the actual
PID/start ticks and before/after executable device/inode. External creator SIGKILL uses
the fixture-only HCLF0741 four-right carrier and an owned subreaper; exact orphan
pidfds are killed/reaped and the previous subreaper setting restored. Foreign
same-UID sending uses a stack/syscall-only fork child, not post-fork Rust logic.
Other cuts cover both guardians, queued guard failure, expiry, socket loss,
SIGKILL held/after exec, lost confirmation, malformed packets/rights, duplicate
release, executable/config/stream/pidfd substitutions, missing seals and exec
failure. The changed-alarm case retargets only its private fixture alarm and
requires refusal against the original bootstrap deadline.

## Scope and replay

HCHL0741, HCLR0741 and fixture-only HCLF0741 are separately registered. All 438
preceding runtime files and old schemas/registry bytes are reconstructed exactly
after removing only the two new format/registration blocks. Exactly two named
runtime sources are added; old worker policy/recipe/locks and original live
rounds/46 brackets/300-60-10 origins are preserved. The full 321 contracts and
joined original live/SIGKILL matrix are not rerun by this additive package.

Ready is preparation, not guardian Armed or exec ACK. EOF remains uncertain;
the owned fixture's release latch never rearms. Guardian pins are liveness only,
not all-writer termination. Post-exec original authority, active collector,
durable completion/later cells and reconnect are not implemented by this helper.
No original/system-bus send, server/install, supervisor, production enrollment,
product workload, expensive qualification or clock change occurs.

After sealing, verify.ps1 checks membership, hashes, source/recipe/image/count and
exact case/seed bindings and 32 in-memory receipt metadata refusals. Do not add
outputs here or reseal the packet. Rerun final/checks.ps1 only in a fresh, equally nested output copy
and clean selected checkout; it refuses to overwrite logs. Throughput and
allocations are not measured by this nonproduct lifecycle fixture.
