# Signed mapping and original namespace credentials local evidence

This packet records scoped local safety, not production enrollment or release
qualification. Baseline `4fa462b2cd339f6e4bab4e7371bcdaa8125dea98`;
preregistration `c83f320ea5b14a9929b83f686bcf2d0db608cd5c`;
implementation `72322696b422f3621763fb2af1c9535c0f1d9a7c`, tree `b42f5798bfde51d1b26f3db25be5e7e335b633bf`.

The [design](../../diagnostic-policy-credentials-local-design.md) fixes exact UID,
primary GID and supplementary-list comparison, without primary union or new
numeric policy. It borrows original context/account and namespace-checked readers,
then checks account/credentials/account. Both original brackets remain intact.
Prior refusal or mismatch precedes IO; first typed error stops and refuses the
original signed policy, account/context and both credential components. Fixed
and fixture origins stay distinct and neither Send nor Sync. Healthy drop leaves
inputs healthy; failure and restoration/drop never reset them.

Baseline Windows: 43 passed. Missing API before implementation: ordinary E0432,
WSL wrapper exit 1, not an ICE. First green: 14 binding cases; review adds one
fixed-negative binding and one component/projection case. Final Linux: 324
passed, one existing system-bus ignored (250 library, 29 artifacts, 9 context-files,
2 manager, 3 account, 5 context, 13 files, 13 policy). Windows: 43 portable passes;
Linux-only context-files executes zero tests. Root contracts/evidence/governance:
132 passed; new root guard also passes Linux. Fifteen binding and one component
case pass three separately captured repetitions. Seed `0x7562026` varies 256
modeled numeric mapping mutations per run, not real host policy re-enrollment.

Initial linux-tests/linux-check captures retain an unused-mut warning in a new
test. The redundant annotation was removed; linux-tests-final/linux-check-final
and strict lint pass. No test failure, threshold change or product hypothesis
rollback is involved. Logs retain both initial and final gates; final claims use
the final captures. docs.log precedes the final documentation/registry; docs-final
and registry reflect completed documentation.

Tests run Rust 1.94.0 --locked, jobs=2, dev/test debug=0, incremental=0, as WSL
unprivileged artur and Windows portable checks. Linux supervisor --lib is joined
with diagnostic_artifacts/context_files/manager/worker_account/worker_context/
worker_files/worker_policy targets. Root tests are xtask performance_contract_074,
release_evidence, release_governance. Check and strict clippy use supervisor+xtask,
--all-targets; clippy --all-features -D warnings. Scoped rustfmt, docs, local
performance contract, links/book pass; require-ship is expected native exit 1.
No full workspace milestone gate is claimed.

[manifest.json](manifest.json) pins all raw log bytes and hashes. Run
`python3 verify-packet.py` for read-only verification. Frozen qualification
manifest and contract inputs remain unchanged.

Actual owned NNP cat helpers and local context combine with synthetic signed
account fixtures and test process-generation pins. A private fixed-wrapper
negative uses a deliberately invalid origin only to prove prior refusal blocks
IO; public APIs cannot convert fixture origin, and no valid production account
is inspected. The binding accepts an already namespace-checked reader; it does
not retroactively attest status opening under signed mount/host context, worker
mount namespace, kernel file-opener credentials or all threads. No real issuer,
initial-host attestation, atomic/continuous snapshot, IO deadline, durable
epoch/refusal, production preparation, authenticated start, workload, performance
measurement or qualification follows. Next: signed-context bracket at status
opening, then separately reviewed enrollment/lifecycle prerequisites.
