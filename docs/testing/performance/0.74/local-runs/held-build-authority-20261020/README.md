# Purpose-bound held runtime build authority

Baseline a3a6682565132ca83fb9fa5c0360f8f06512b027; selected implementation
48854c1e67b8acf8eaf7a36fd2381ad2b52ac9ed. Inherits docs/RULES.md and the
[approved local plan](../../held-build-authority-plan-20261020.md).

This packet checks a portable signed-data/content component. Fixtures contain
synthetic keys, ELF headers, lock bytes and invented Cargo events. The test images
recorded here are integration test executables, not authenticated role images.
No held launcher target exists yet and no real held-runtime receipt was issued.

## Baseline and refusals

The old Windows artifact selection passes seventeen tests before the change.
The preceding boundary packet verifies unchanged. A missing-module compile
refusal precedes implementation. The preliminary source guard build then refuses
seven helper visibility errors; that harness import defect is corrected without
changing a runtime assertion. Both refusals and the accepted development logs
remain, rather than being overwritten by final selected-source results.

## Selected-source checks

checks.ps1 requires the supplied exact HEAD, clean runtime/test/xtask/lock/COMPAT/
registry/qualification inputs and unchanged qualification digest before and after
each cohort. It records recipe/compiler identity, selects exact test executables
from Cargo JSON and retains image SHA256 before execution, checking it afterward.
The new receipt's real-build recipe is not run by this test recipe.

The sixteen new cases pass on Windows, current GNU 1.94 and MSRV 1.88. They cover
independent pins and same-controller/distinct-key constraints, exact source and
epoch, canonical bounded formats and domain separation, unsigned and re-signed
binding/observation mutations, wrong ELF/contents/roles, Cargo records and 128
seeded content corruptions (seed 0x0074_b017). The complete cohort receipts also
record old artifact/receipt regressions, two compile-fail capability controls and
strict lint. All four cohort receipts report success at the selected unchanged
HEAD. Accepted ordinary selections are:

| Compiler | New integration | Old artifact | Old receipt | New compile-fail docs |
| --- | ---: | ---: | ---: | ---: |
| Windows 1.94 | 16 | 17 | 8 | 2 |
| GNU 1.94 | 16 | 31 | 9 | 2 |
| GNU MSRV 1.88 | 16 | 31 | 9 | 2 |

Strict lint passes on all three compilers. Repository checks pass eight exact source/adapter tests and three
predecessor fixed-slice bridge controls, fmt, registry/docs and the local
non-promotable contract; the gated registry still discovers 184 registered gates.
The three preceding sealed packets verify unchanged. Their replay logs here do
not claim rerunning their optional metadata mutation controls.

The source guards reconstruct every one of the 437 old runtime sources after
removing exactly one builder hook and one registered COMPAT block. They allow
exactly the one new diagnostic_held_build.rs runtime file, rejecting omitted or
extra sources and missing/duplicate adapters. Previous worker schemas, source,
recipe, keys, locks, test bodies, registry and all original live rounds remain.

## Scope and replay

Static local topology approval is recorded separately from false actual-role-
build, production-enrollment, original-join/send, server, supervisor, product,
clock-change and new-live/fault-matrix fields. The full 321-contract suite and
kernel/live/SIGKILL matrices are not rerun or claimed by this pure additive package.
There is no performance measurement or numerical product claim.

After sealing, run verify.ps1 for exact membership, hashes, receipt/image/count
bindings and 26 in-memory receipt metadata refusal controls. Do not add replay
outputs to this sealed directory or reseal it. To rerun checks.ps1, use a fresh
copy/output directory and the selected clean source; the script refuses to
overwrite existing logs. Production role builds, retained executable objects,
actual held-launcher/worker exec and the complete original live join remain next.
