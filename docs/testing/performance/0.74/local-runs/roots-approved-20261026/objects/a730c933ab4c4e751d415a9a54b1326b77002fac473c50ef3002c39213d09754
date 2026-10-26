# Exact public root catalog approval

The operator approved the complete upstream 1.0.10 public catalog on 2026-10-26,
conditional on exact key/constraint review. `approved-catalog.json` records that
scope: 116 surviving complete anchors, two removed roots and four added roots.
It is not a production issuer key, enrollment policy, tenant permission or server
admission. The historical narrow removal-only refusal remains immutable.

`audit.py` reads the pinned earlier packet as data, parses complete anchor/DER
multisets and maps changed certificate subject/SPKI fields structurally. Full
SPKI and certificate fingerprints must match all six approved entries. Every
surviving anchor constraint and DER certificate remains exact. Added roots have
critical CA=true BasicConstraints and no NameConstraints. This is catalog audit,
not independent CA issuance review or a certificate-chain exploit reproduction.

The optional `--registry PATH` checks actual cached `.crate` checksums, source,
manifest, VCS metadata and license against archived published bytes. Tar members
are read, never extracted or executed. `validate_graph` accepts only the two
catalog updates and three dev-only regression edges relative to supplied inputs.
No Git, network, process, clock, service or enrollment action is performed.

```powershell
python tools/roots-trust-admission-074/audit.py
python -m unittest discover -s tools/roots-trust-admission-074 -p test_audit.py -v
cargo +1.94.0 test --offline --locked -p hydracache-redis-compat --test tls_roots_security_074
```

Both test dependencies are exact `=1.0.10`; future public-root additions need
another scope decision rather than an unreviewed resolver selection. The legacy
0.26.11 test edge must expose the same resolved parent catalog. The five Rust
test bodies are byte-identical to the archived corrected oracle, including the
registered TUBITAK out-of-band `.tr` anchor constraint difference.

The [acceptance report](../../docs/testing/performance/0.74/roots-approved-result-20261026.md)
separates current combined-graph checks from the earlier pre-Tokio candidate,
full package coverage and production trust/lifecycle admission. Old source-view
and package-review pins intentionally refuse this new dependency graph until a
separately reviewed exact successor bridge is implemented.
