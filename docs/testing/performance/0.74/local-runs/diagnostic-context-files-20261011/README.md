# Context-bound account opening: local safety evidence

This packet records baseline, tests-first refusal, implementation and scoped
working-tree safety gates, not release qualification. Baseline
`81a3a5574121ea807218c23c5694c723dd6e71d8`; preregistration
`99abe16df6df3f4bcb8213169914ee69aeb3cd60`; implementation
`fb8ddc733d0815c403a47948ee04a1fe2be9b820`, tree `77576c165e4112a7e0ea69a74acb13a147569964`.

The [design](../../diagnostic-context-files-local-design.md) fixes opening order
context/open/context/files/context and later context/files/context. Original
context, FileSet and borrowed policy are retained; the first error stops reads
and permanently refuses the policy through restoration/drop. Fixed and owned
fixture origins stay distinct. Existing standalone readers, parsers, limits and
signed schemas are unchanged; no production route is added.

Local results: Linux 308 passed, one pre-existing system-bus case ignored;
Windows 43 passed (new Linux target zero tests); root contracts/evidence/governance
131 passed. Nine new integration and four unit cases pass three separately
captured repetitions. Seed `0x7552026` covers 64 valid re-signed foreign context
values per run. Scoped check/lint, format, documentation and new Linux root
contract pass. Require-ship is expected native exit 1; frozen qualification digest
is unchanged. Missing API before code is E0432, WSL wrapper exit 1, not an ICE.

Commands use Rust 1.94.0, --locked, jobs=2, dev/test debug=0 and incremental=0.
Linux runs as unprivileged WSL user artur with the existing local target cache;
Windows portable gates use the existing worktree target. The final Linux scope
is supervisor --lib plus diagnostic_artifacts, diagnostic_context_files,
diagnostic_manager, diagnostic_worker_account, diagnostic_worker_context,
diagnostic_worker_files and diagnostic_worker_policy. Root scope is xtask
performance_contract_074/release_evidence/release_governance. Both package check
and clippy use supervisor+xtask --all-targets, clippy --all-features -D warnings.
No full-workspace gate or infrastructure campaign is claimed.

Raw logs are copied byte-for-byte after each capture closes and pinned in
[manifest.json](manifest.json). Initial repeat capture names lost the shell
iteration suffix and retained only the final successful iteration in
repeat-integration-.log/repeat-unit-.log; these limitations are retained, not
used to establish three repetitions. Six explicit numbered captures independently
establish the claimed three repetitions. docs/contract/links/book logs precede
final documentation; docs-final and registry capture the completed registry/docs.
Run `python3 verify-packet.py` from this directory for read-only verification.

Positive fixtures combine actual Linux context with owned temporary account files
under synthetic issuer pins, not a valid production worker account. No real issuer,
initial-host attestation, atomic snapshot, file-opener credential attestation,
continuous revocation, file IO deadline, all-thread proof, durable epoch/refusal,
production preparation, authenticated start, workload or performance result is
established. Next: kernel-credential join; lifecycle and qualification stay closed.
