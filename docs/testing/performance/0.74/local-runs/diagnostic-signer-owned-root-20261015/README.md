# Fixed normal GNU root preparation: local development evidence

Preregistered in `7c27c3c6b66efdcefa61b916bfe3210852354255` against baseline
`81c327d1fed7a2284d4ebd96a8e80f2961e0787b`. Implementation
`97e1bde1481394b387b47bd99497afb2d2b8ac93`, tree
`81d80384fbdce09fbe0c7ba45b55ff39a821f1e6`, adds binary-private normal root
materialization, CLI/tests and closed contract/registry guard. The ordinary
owner, isolation, GNU parser, old root/constructor fixtures and helpers are
unchanged. Frozen Cargo.lock/qualification manifest are pinned unchanged.

Final Linux Rust1.94 passes 11 unit, one CLI and three normal-process tests.
Actual private stripped self plus real fixed loader/libc/libgcc are materialized,
rechecked and explicitly removed, including four independent normal processes.
No child is launched inside the root. Synthetic unit graphs must not be counted
as executable runtime validation. The fixture also covers eight owned partial
prefixes, collisions, source/copy drift, CLOEXEC loss, hardlinks, foreign file/root
substitution, symlink and extra-entry refusal before cleanup mutation.

Rust1.88 passes the same test counts with a different process outcome: its local
normal executable declares extra libm, independently observed by readelf, and all
five otherwise-positive preparations deliberately refuse. Its 11 synthetic/unit
cases pass. The initial expectation of runtime success failed 1/2 in process tests
and remains retained. The four-role parser/limit was not expanded. Windows current
and MSRV each pass one portable CLI-refusal test. Check/strict lint and formatting
pass; root contracts pass159, registry175, doc/local-contract pass, ship refuses.
The MSRV extra-libm profile is not supported root execution.

Other retained failures are a unit symlink setup missing write permission for a
directory move, and a source-rename test incorrectly assuming that renaming a path
must change original metadata. The latter now preserves exact original stamp and
byte checks without claiming source pathname custody or recapturing a replacement.
A concurrent Windows xtask build collided with its running executable (OS5);
sequential validation passed. These are not production fixes or permission waivers.

The broad unchanged library baseline failed78/1 in
`signer_constructor_loss_seeded_fresh_and_independent_observers`, with an inner
constructor-death refusal and owned-root refusal context. Later focused root and
constructor groups pass7/7 each; GNU/normal-owner regressions pass6/12/5. This does
not attribute or fix the broad failure. All-regressions-passed remains false;
investigation precedes further launcher composition/server admission.

Sixteen captures retain failures and final checks. Capture text is tool-returned,
CRLF-normalized and terminated with LF when saved; the manifest/verifier pins those
saved bytes, exit codes, exact summaries, required markers, source/tree, unchanged
oracles and frozen inputs. This is unsigned development consistency, not proof
that commands executed a clean checkout at the final commit, execution attestation
or independent source/runtime provenance. `verify_packet.py` plus 15 verifier
tests (positive and mutation cases) pass on Windows and Linux. Run from this directory:

```text
python -B verify_packet.py
python -B -m unittest test_verify_packet.py
```

No credentials, token/signing, server/GitHub operation, product workload, full
release gate, qualification or performance measurement was performed. No admission
or performance claim follows. See the [design](../../diagnostic-signer-owned-root-design.md)
for explicit cleanup and future composition boundaries.
