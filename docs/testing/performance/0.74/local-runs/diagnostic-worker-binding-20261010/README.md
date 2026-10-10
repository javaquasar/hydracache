# Asserted worker binding local evidence

This packet records local mapping consistency and sticky refusal for the fixed
account and original kernel credential guards. It is not trusted host enrollment,
production execution, clean-source qualification or numerical performance evidence.
See the [design](../../diagnostic-worker-binding-local-design.md) for the exact
group policy and test boundary.

The [manifest](manifest.json) binds 24 raw captures by size and SHA-256. Run
`python verify-packet.py` from this directory to check retained bytes, observed
test counts, negative results, expected-red ship admission and the unchanged
qualification digest. The verifier is read-only and launches no worker or host
operation. It verifies the local evidence claims, not the authenticity of NSS
configuration or a production start.

The Linux baseline passes 205 tests with one pre-existing ignored. Initial API
red precedes twelve green binding tests. Final and reviewed suites pass 218;
thirteen new binding tests and three existing account integration tests pass
three repetitions before and after the lint-only test correction. Windows
passes 28 portable tests and no Linux binding cases; root contracts, evidence and
governance pass 124 tests before packet registration, followed by one registry
check. Linux strict lint first rejected explicit non-Drop wrapper drops; lexical
scope correction preserves the same behavior and leaves production code intact.

Positive binding tests combine actual owned NoNewPrivs cat kernel readers with
synthetic fixed-account snapshots and a test-only process pin. Public-constructor
tests establish early refusal and pending-child retention, not a positive
production path. The separate real NSS operator check refuses with exit 9,
empty stdout and confirmed cleanup; its underlying cause is not exported.
No host account was created, no systemd unit changed, and no product workload
or qualification ran.
