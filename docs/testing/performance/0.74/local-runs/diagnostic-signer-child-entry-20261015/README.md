# Ordinary fixed child entry: local enforcement evidence

Preregistration `7fd7a45608f9175be81c8ddb35b060d0a87080d4` against baseline
`a4a3c1b78055bbf6c6a2495c00950a4a3a7b021d`. Implementation
`41cb929af58ed8e81d6083710832aa3513ab9bfe`, tree
`9242b7358b381aba89bf8628e97a37686c9d8df5`. See the
[design](../../diagnostic-signer-child-entry-design.md) and
[contract](../../diagnostic-signer-child-entry-contract.toml).

This is an unsigned local consistency packet, not a signed execution receipt,
production trust root or performance result. Captures were collected during
development before the final implementation commit; baseline/red/temporary
diagnostics intentionally are not executions of that final source. The manifest
binds the final reviewed source files, frozen inputs, old independent oracles
and normalized UTF-8 merged tool-output captures (LF, one terminal LF). It does
not cryptographically attest execution, clean-tree provenance or dedicated-server
compatibility. Only `clippy-ice-partial.log` is incomplete: the tool truncated the
long diagnostic. Its outer failure and the subsequent complete short diagnostic
are preserved; the partial trace is never counted as a passing check.

## Results and failures retained

- Baseline: seven portable context tests and 13 old namespace tests. Missing
  ordinary API fails E0432 before implementation.
- Initial ordinary tests passed, but repeated Linux/MSRV processes later refused
  exit2. A temporary fixed-stage diagnostic located preflight FD refusal, then
  bounded inventory showed {0,1,2,3,7}, with 3 the reader's own directory. FD7's
  origin was not authenticated. Those failed runs remain; no guard was relaxed.
- The test-only owned launcher now requires a child-only close_range CLOEXEC
  sweep before exec. The explicit inherited-FD negative clears its private copy's
  flag after that sweep and still refuses. Temporary normal-code diagnostics
  are removed. Parent descriptors/restrictions are not changed.
- Eight final Linux Rust 1.94 repeats each pass five normal-process and seven
  portable policy tests. Each process suite checks 20 exact native EPERM calls,
  unfiltered chdir success, x32/i386 SIGSYS without core/output, environment/FD/
  argument refusal and eight native children under four independent owners.
- Three Linux decoder unit tests pass; root/non-single-thread negatives are
  parser models, not privileged root or actual multithreaded signer runs.
- Windows Rust 1.94/MSRV 1.88 portable suites pass seven each. Linux MSRV all-target
  check plus unit/process/model suites pass 3/5/7. Strict all-target/all-feature
  Clippy passes on Windows and Linux; Windows also checks xtask. Format passes.
- Old namespace/rooted-exec/handoff regressions pass 13/11/13. Namespace helper
  SHA256 remains `bf78cb987e98962676c63cf62f3efae0ecf0cd7258f126a231634bc41d075081`.
- Root performance contracts pass 156. Registry passes 173 discovered gates;
  docs and local performance contract pass. Ship admission deliberately refuses
  exit1; the capturing wrapper requires that refusal and exits zero.
- A registry spelling mismatch initially failed discovery and was fixed to its
  normalized cfg spelling. Initial annotated Clippy panicked rendering its lint;
  complete short output identified manual alignment modulo. Equivalent bounded
  bit alignment replaced it, then the unchanged strict lint passed. No lint
  suppression or threshold change was added.

Observed unsigned normal probe hashes after final checks: Rust 1.94 debug
`8703a7c563b0e52670a6315305bdcdaaa12df9425c3c47605c3490df95c520e8`,
Rust 1.88 debug
`db62d210b51ed532dd6586398e2c7cc480017b7f37c0785d25e461855c1ef172`.
They identify only local artifacts, not reproducible-build or admission receipts.

## Reproduce

From this worktree, use the regular test harness, not the process-terminating
library function in an existing application. Commands need installed dependencies
in the offline cache; no server, sudo, key, token or external workload is needed.

```text
cargo test -p hydracache-long-run-supervisor-074 --test diagnostic_signer_isolation --locked --offline
# Nonroot Linux x86_64 only:
cargo test -p hydracache-long-run-supervisor-074 --test diagnostic_signer_child_entry --locked --offline -- --nocapture
cargo test -p hydracache-long-run-supervisor-074 --lib diagnostic_signer_isolation::linux::tests --locked --offline
python docs/testing/performance/0.74/local-runs/diagnostic-signer-child-entry-20261015/verify_packet.py
python -m unittest discover -s docs/testing/performance/0.74/local-runs/diagnostic-signer-child-entry-20261015 -p test_verify_packet.py
```

Fixed policy bytes are real installed restrictions in these fresh owned probes;
they are not a complete sandbox. No selected root, authenticated original exec,
protected job/runtime custody, real credentials, all-writer revocation, completion,
later cells, signing or server qualification follows. The normal controller/server
does not consume this API. Next: compose an original-owner launcher with selected
root/runtime and explicit descriptor/parent-loss lifecycle before any credential
handoff. Frozen lock/qualification manifest, product and old helper sources remain
unchanged. No server/GitHub state, service, account or campaign was touched.
