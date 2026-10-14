# Local public OIDC acquisition verification

Implementation `b877d014c92c7eb57eaed2afb5791925374358ca` adds the fixed public
HTTPS tool, bounded isolated child, unsigned report and ordinary CI fixture step.
Baseline is `e007113d52a6031e3254c36fa169f495ddf56e08`; preregistration is
`78b114488434d21334d9ed6382b8056e46402fdf`. See the
[design](../../diagnostic-oidc-acquisition-design.md),
[contract](../../diagnostic-oidc-acquisition-contract.toml) and
[operator procedure](../../diagnostic-controller-producer-operator.md#public-acquisition-tool-after-separate-approval).

## Local results

All 12 acquisition cases pass in three final repeats on each platform with
Python warnings treated as errors. The combined builder/offline/acquisition
suite passes 25 cases on Windows Python 3.13 and Linux Python 3.12. Earlier
captures have ten or eleven cases; the final strict captures cover the final
source, including report-type equality and modeled CLI success/budget refusal.
Seed `0x07572074` drives 32 report mutations and four independent models.

Each final repeat makes three real owned loopback TLS observations: synthetic
accept, wrong hostname and unknown CA. Only the test context trusts the public
synthetic CA. A real owned blocking child times out and is reaped without
accepting partial output. Its private fixture deadline is 0.5 seconds; the
production 15-second subprocess and five-second resource limits are unchanged.
This is process-timeout evidence, not a durable-publication SIGKILL matrix.

HTTP/report tests cover fixed headers/origin, status/redirect/compression and
duplicate/framing/length/budget refusal, certificate/deadline failure, malformed
discovery before the second fetch, job clock reversal, environment refusal before
launch, minimal child environment, report drift and zero-versus-false confusion.
The Windows development failure preserves the TCP-reset/TLS-alert distinction;
the corrected test still requires client certificate verification refusal.
Missing API and every functional negative are retained.

Root regression passes 131 performance contracts, six registry cases and 13
release-evidence cases. The final updated ordinary-CI source guard also passes
separately. Affected current all-target check and strict Clippy pass; the Windows
MSRV 1.88 contract target check passes. Format, documentation consistency, 169
registered gates, local performance contract, local links and book generation
pass. `--require-ship` retains expected exit 1. No full release gate, remote CI
receipt, numerical workload, new Rust runtime change or dependency change is
claimed by this bounded tooling slice.

## Reproduce without actual acquisition

```sh
python -W error -m unittest scripts.perf.test_diagnostic_oidc_acquisition_074
python -W error -m unittest scripts.perf.test_diagnostic_builder_074 scripts.perf.test_diagnostic_oidc_source_074 scripts.perf.test_diagnostic_oidc_acquisition_074
python docs/testing/performance/0.74/local-runs/diagnostic-oidc-acquisition-20261014/verify_packet.py
```

Do not run `acquire-public` to reproduce this packet. The tests redirect only
owned synthetic connections to loopback or model the worker/HTTP boundary; the
actual GitHub issuer is never contacted. The fixtures are publicly committed
test material and must never be installed in a system CA store or used as keys
for a protected job, policy or deployment.

The [manifest](manifest.json) pins exact committed source, frozen locks in both
original Git sources, unchanged standalone inspector/controller/supervisor and
qualification inputs, plus all raw captures. Verification reads files and Git
objects only. Windows-managed worktree metadata requires the exact translated
`GIT_DIR` for WSL read-only Git operations; never rewrite metadata or switch the
main checkout. Historical source matching remains exact.

## Remaining admission decisions

The report describes HTTPS under platform CA trust, not independently selected
issuer policy. Actual acquisition, audited source/interpreter/CA custody,
explicit key/workflow/subject/snapshot review, protected-job token production and
original replay/common fence remain open. No actual private signing key, token,
trust config, protected job, server operation or product qualification was used.
Original start, watchdog/all-writer cessation, completion and later cells still
require their independent original-owner gates.
