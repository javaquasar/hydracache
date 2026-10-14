# Local diagnostic producer verification

Implementation `fdc3bb5d350df6a77bb78ec330bc63b4c238ddd8` adds a separate
job-side canonical request/audience/signature producer and offline public-source
syntax inspector. Baseline is `fc696e72aa33e5a5d41227f9314455293705a9fb`;
preregistration and the frozen Python fixture correction are in `0c7fe697`.
See the [design](../../diagnostic-controller-producer-design.md),
[contract](../../diagnostic-controller-producer-contract.toml) and
[local procedure](../../diagnostic-controller-producer-operator.md).

## Results and reproducibility

The final six Linux producer cases pass in three explicit repeats. They cover
strict canonical request/build binding, the existing signature domain and digest
audience, selected-key/time/identity refusal, private file metadata/links/bounds,
exclusive output, validation before secret access, and seeded signing for the
four diagnostic operations. Seed `0x07562074` drives 32 synthetic request cases
and four independent preparations. Public production selection refuses the
synthetic controller; successful file signing uses a private test seam only.

Linux integration passes 24 IPC and five kernel-binding/CLI cases. Regression
passes ten OIDC cases, one existing builder-file case, 29 artifact cases and
eight reviewed-trust cases. All 85 compile-fail doc tests pass. Windows passes
four producer, 21 IPC, 15 artifact and eight reviewed-trust cases. Python passes
seven builder and six offline inspector tests on each platform, including 32
seeded malformed public integers and read-only CLI behavior.

The root suites pass 130 performance contracts, six registry tests and 13
release-evidence tests. Current all-target checks and strict all-feature Clippy
pass for affected packages; supervisor all-target MSRV 1.88 checks pass on both
platforms. Formatting, strict Linux rustdoc, doc consistency, all 169 registered
gates, local performance contracts, link checking and book generation pass.
`--require-ship` retains its expected exit 1. The full release gate and Linux
MSRV Clippy were not executed for this bounded step.

Twenty-one raw captures retain all functional negatives: the frozen Python fixture
regression, missing API baseline, missing metadata trait, private test imports
and a SHA formatting assumption, plus two WSL Git-context failures. Those fixes
neither relaxed runtime validation nor exported private helpers. Earlier green
captures precede the final extension
to all four operations; `final-four-operation-repeats.log` and
`windows-final-producer.log` validate the final source. No timing result here is
a product performance measurement.

Run from the worktree containing this exact source:

```sh
python docs/testing/performance/0.74/local-runs/diagnostic-controller-producer-20261014/verify_packet.py
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --locked --lib producer_ -- --nocapture
cargo +1.94.0 test -p hydracache-long-run-supervisor-074 --locked --test diagnostic_ipc --test diagnostic_worker_kernel_binding
python -m unittest scripts.perf.test_diagnostic_builder_074 scripts.perf.test_diagnostic_oidc_source_074
```

The [manifest](manifest.json) pins exact committed source, unchanged tooling and
signed inputs, frozen locks in both original Git sources, qualification raw hash
and every raw capture. The verifier is read-only; it performs no network, key or
host operation. Source matching is exact, so later changes require a separate
packet rather than rewriting this historical evidence. A Windows-created worktree
contains a Windows Git-directory path; WSL read-only verification must use its
exact translated `GIT_DIR`, as retained in the final capture, rather than modifying
worktree metadata or switching the main checkout. Native Windows verification
needs no such translation.

## Remaining admission dependencies

Producer output is signed job data, not original receiver admission or consumed
nonce. Offline discovery/JWKS inspection authenticates no origin and selects no
production key/workflow/expiry. Actual authenticated acquisition and custody,
protected-job policy/token production, trusted host/worker policy/epoch and
original replay/common fence remain before authenticated start, watchdog/writer
cessation, completion and cells two through four. No actual private signing key,
token, installed config, workflow dispatch, server operation, new SIGKILL proof,
product candidate or qualification workload was used in this slice.
