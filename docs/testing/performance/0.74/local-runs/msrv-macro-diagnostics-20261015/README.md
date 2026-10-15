# Macro MSRV diagnostic cleanup

This local step removes the seven Rust1.88 `uninlined_format_args` errors in
`hydracache-macros` recorded by the preceding
[send-window packet](../diagnostic-signer-owned-root-send-windows-20261015/README.md).
It changes diagnostic construction, not parsing or runtime caching behavior.
The wider Windows MSRV lint remains red with three existing format errors in
`crates/hydracache/src/grid/conditional.rs`; this is not an all-checks-green
receipt or server-admission evidence.

## Baseline and implementation

The baseline is `e83b3ecb0cc0c4c1c95c4b73b20f007cb5b8f833`. Its macro suite
passes 76 tests while strict MSRV Clippy fails on seven format expressions.
Seven exact-message characterization tests are added before changing those
expressions: duplicate cacheable expression/segment options, duplicate query
expression lists/types, missing list commas, empty lists and duplicate prepared
types. The resulting 83 tests pass on the old implementation. The retained
`test-first.patch.log` contains only that test addition, including the formatter's
layout, and `test-first-msrv-tests.log` records that pre-fix run.

The implementation replaces exactly seven positional format arguments with
implicit captures in `cacheable.rs`, `policy.rs` and `prepared_policy.rs`. Error
spans, parsing conditions, generated successful expansions and accepted input
rules are not changed by this diff. Existing tests remain in place; there is no
lint suppression, dependency update, threshold change or runtime optimization.

The source commit is `36f284eac5a1c58796cc22613e064bc15d331988`, tree
`530ff3b237fa080e340251a3fb94b81c3d295a31`. The main comparison checks execute
before that commit on the same source files; separate post-commit Windows macro
tests and MSRV lint are captured as `postcommit-*`. This is not a clean-worktree
or signed execution receipt.

## Results

| Check | Result |
| --- | --- |
| Macro tests on Windows and WSL Linux with Rust1.88 and Rust1.94 | 83 pass in each of four runs |
| Affected macro/supervisor/xtask checks, all targets/features, both platforms/toolchains | Pass |
| Strict Clippy for macros, all targets/features, Windows Rust1.88 | Pass |
| Wider strict Clippy for macros/supervisor/xtask, Windows and Linux Rust1.94 | Pass |
| Wider strict Clippy, Windows Rust1.88 | Fails on three existing conditional error format strings |
| Linux Rust1.88 Clippy | Not run; component is absent |
| Linux Rust1.94 runtime macro tests and compile-pass/compile-fail snapshots | Pass |
| Linux Rust1.94 DB derive/query/prepared-policy compiler snapshots | Pass |
| Windows package formatting and Linux whole-workspace formatting | Pass |
| Earlier send-window packet consistency verifier | Pass; unsigned, not execution attestation |
| Static performance contract tests and gate registry | 165 tests pass; 175 gates registered |
| Local performance contract | Pass, non-promotable |
| Release documentation consistency | Pass |
| Require-ship admission | Expected exit1; candidate identity and qualification remain incomplete |

The initial combined Windows inspection ends with a whole-workspace formatter
failure (`os error 206`, filename or extension too long). Its full output is
retained in `windows-workspace-format-attempt.log`. The unchanged formatter rules
then pass for the affected package on Windows and the complete workspace in WSL;
this workaround does not make the original Windows command successful.

Commands and complete captured output are indexed in [manifest.json](manifest.json).
Logs retain tool-returned text and ANSI controls, normalized to UTF-8/LF with one
final newline for nonempty captures; empty successful formatter logs remain empty.
The capture digests detect file changes, not forged execution. These commands run
in the existing worktree, not an independently attested clean checkout. Older
packets and their failed outputs are not edited or retrospectively promoted.

To repeat the focused macro gate with an installed toolchain:

```sh
cargo +1.88.0 test -p hydracache-macros --locked --offline
cargo +1.88.0 check -p hydracache-macros --all-targets --all-features --locked --offline
cargo +1.88.0 clippy -p hydracache-macros --all-targets --all-features --locked --offline -- -D warnings
```

Substitute `+1.94.0` for the current-toolchain comparison. The manifest records
the actual WSL working directory, cargo executable and separate target directory.
The broad Windows MSRV failure is a remaining gate, not an expected-red canary.
The separate `verification.txt` is outside the capture manifest; the final local
check matches all 31 capture lengths/digests, source blobs/tree and frozen files.

## Remaining work

The next code-quality package can preserve and fix the three conditional error
diagnostics, then rerun the wider MSRV lint to expose any further dependency
failures. Linux MSRV lint needs its missing component before it can be claimed.
No allocation, throughput or native-versus-RESP performance measurement is made
here. Authenticated original-start, trusted runtime/issuer/job custody, P2/P3
all-writer cessation, completion/later cells and server admission remain separate.
No server, GitHub workflow, qualification workload, release merge/tag or push is
performed by this packet.
