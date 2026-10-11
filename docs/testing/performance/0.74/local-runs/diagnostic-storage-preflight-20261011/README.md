# Local server storage preflight checks

The [design](../../diagnostic-storage-preflight-design.md) resolves the common
parent ownership mismatch and implements a fixed read-only Linux CLI. This packet
is local code evidence, not an installed server receipt or worker-start permission.

The final selected Linux scope passes 451 tests with one existing ignored.
The four new preflight unit tests also pass three serial repeats; eight
compile-fail doctests pass. Provisioning and CLI regressions cover unsafe-existing
metadata refusal before installation and nonroot/extra-argument refusal. The
actual private walkers are exercised in nonroot temporary trees; GID drift is
synthetic retained-stamp mutation, not privileged chown. Static installer checks
and bash syntax validation do not execute installation or systemd.

The portable Windows supervisor run passes 196 tests. Root checks pass 109
performance contracts, 13 evidence, six gated-target and 23 governance tests.
Strict lint on both platforms, MSRV 1.88, rustdoc, formatting, repository doc-check,
book and the non-promotable local contract pass. The broad root contract first
refused the CLI import of the output-preparation namespace; a dedicated read-only
facade fixes that route without weakening the existing no-preparation check.
The Linux scope is repeated after the facade change at the final implementation
source. Earlier Windows counts describe the pre-facade source (the changed facade
is Linux-gated); final portable lint/MSRV still check the final source.

The initial shell repeat loop retained only its last overwritten `repeat-.log`;
that capture is not three-repeat proof. Three subsequent commands use distinct
literal filenames and retain all three successful four-test repeats.

The manifest pins baseline and implementation Git identities, source hashes,
closed raw logs and historical evidence. Historical source is checked at its
original commit, not claimed equal to amended current code. The frozen
qualification input mismatch remains an expected negative result; local contract
success is non-promotable and ship stays closed.

Verify from the repository with native Python:

```text
python docs/testing/performance/0.74/local-runs/diagnostic-storage-preflight-20261011/verify-packet.py
```

For controlled host prerequisites and the exact command, see the
[server instructions](../../production-readiness-and-server-launch-20261011.md#read-only-server-storage-check).
No server operation, issuer enrollment, worker execution or cache performance
measurement is included. The fixed publisher, original fenced state/authority
composition and production lifecycle remain the next code boundaries.
