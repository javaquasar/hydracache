# Build the exact frozen observer locally, without a workload

This new slice allows a local Linux release build of observer `62114be0`, not a
candidate, host install, signature enrollment or product workload. Earlier
verification-only contracts/packets remain immutable and still describe their
own narrower slice. The reviewed `BUILD_COMMAND` in `diagnostic_artifacts.rs`
is executed unchanged with Rust 1.94.0, empty features and its ordinary target
path. No allocation instrumentation, RUSTFLAGS or profile overrides are used.

The dedicated detached build worktree must have the exact commit/tree and clean
status before/after compilation. Checkout bytes of locks/configs must match
their frozen hashes; Windows CRLF checkout conversion is not acceptable for
include_bytes build identity. The first newly created, clean build checkout had
converted lock bytes and was replaced with a LF checkout before any compilation.
No existing product or qualification worktree was changed.

Retain native Cargo JSON stdout separately from compiler stderr. Check its one
unprofiled timing artifact and final success, bounded ELF structure, locks and
four P0 configs. Run the compiled binary only with `--validate CONFIG` on each
surface; require valid=true, fixture_started=false, admission_allowed=false.
No --run invocation, fixture, latency/CPU/allocation claim or trusted build
receipt is produced. Source identity is independently checked by the local
coordinator, not authenticated by the binary's validation output.

This reduces build-readiness uncertainty. A local developer build and raw Cargo
log are not trusted builder attestation or independently reproduced compilation.
Signed enrollment, live backend, policy/process/tree/artifact binding, writer
revocation, watchdog and uncertain-intent reconciliation still precede launch.

## Read-only content audit boundary

The local audit will expose only bounded content inspection: ELF structure,
native Cargo JSON, lock bytes and the four frozen P0 config hashes. Its result
must be explicitly unsigned, source identity not proved by the result, and
admission false. It cannot construct VerifiedBuild or issue a build signature;
the authenticated verifier and production policy remain separate.

Cargo's [package-ID grammar](https://doc.rust-lang.org/cargo/reference/pkgid-spec.html)
allows a version-only fragment for a local path whose last component names the
package. Existing invented logs use a name@version fragment. The content parser
must accept both exact 0.0.0 forms only under the fixed local tool path, while
rejecting wrong paths, source kinds, names and versions. Tests first cover that
distinction; all other profile/feature/artifact/final-event checks remain strict.

P0 configs postdate the frozen observer checkout and are supplied separately
from the current repository by their unchanged hashes, not silently added to
that checkout. Git identity/cleanliness is checked with native Windows Git; the
initial WSL preflight did not reach compilation. Native Cargo stdout remains
separate from stderr regardless of this coordinator choice.

The implemented operator command is `hydracache-long-run-supervisor-074
audit-local-build BINARY CARGO_JSON ROOT_LOCK OBSERVER_LOCK P0_DIRECTORY`. It
reads bounded regular leaves, refuses empty/oversized/linked Unix leaves and
length drift, and produces hashes plus hard-coded false trust/execution flags.
It does not prove an atomic file snapshot, safe ancestry or installed inode
continuity; the root-owned authenticated bundle reader is still required by a
future backend. The result has private fields, no Deserialize and no conversion
to VerifiedBuild. Tests cover the valid synthetic content/CLI path, changed
locks/configs/logs/binary, regular-leaf budgets and local Cargo ID variants.

The historical no-artifact-enrollment root guard initially refused the new CLI
module reference. It now allows exactly one inspect_unsigned_files call in the
operator CLI; every other artifact-module reference in production entry paths,
inspect_fixed_install and BuildTrust remain forbidden. A separate guard checks
the unsigned reader's limits and inability to execute/sign/create VerifiedBuild.
No production key policy, signed diagnostic route or installation is enrolled.

## Actual build and clean audit-source repeat

[The sealed local packet](local-runs/diagnostic-local-build-62114be0/manifest.json)
retains native Cargo JSON/stderr, Git identity and lock/config hashes before and
after, a 64-byte ELF header, the original four validation outputs, both OS audit
outputs, red test-first logs and clean `cee62b37` test/check logs. Observer source
`62114be0` / tree `a9f059d7` remained clean. The actual Linux release binary is
26,492,624 bytes, SHA-256
`2a497dfcf8c5ec8ab32d55fe620ef96c554d55e1e31437f31c6474d2aa6625eb`;
it is retained locally in the dedicated build worktree, not committed to Git.
It was built once, not independently reproduced or enrolled as trusted.

Four --validate calls passed with fixture_started=false and admission_allowed=false.
Real Cargo emitted the version-only local ID anticipated by the red grammar test.
Windows/Linux unsigned audits agree on binary, log and fixed config digests.
Clean audit implementation `cee62b37e978fa29277f58e14a3051eaeaa24c7d`, tree
`63bb3c2e4451b1755e53c14f4d914bde651b38ad`, passed 12+1 Windows and 18+1 Linux
artifact/reader tests, 76 root contracts, 13 evidence and 23 governance tests.
Check/clippy/fmt/docs/local-contract/17 governance CLI checks/links/book passed;
require-ship remained expected-red (exit 1). Packet guards/docs follow separately.

This is build/content readiness only. It proves no workload quality, CPU/op,
latency, retained memory, A/A calibration, native nonregression, full D3 or C74
admission. The next architectural enrollment choice is an explicitly reviewed
trusted Linux builder/key versus reviewed reuse of the provisioning signer's
build domain. No default/test key will be promoted. Backend, policy/tree/process
binding, writer revocation, watchdog/reconciliation and non-product loss rehearsal
still precede signed installation and a baseline-only pilot.
