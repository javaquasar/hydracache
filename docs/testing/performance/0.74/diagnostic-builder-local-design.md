# Separate diagnostic builder authority

The user selected a separate builder key after the actual unsigned local build.
This slice starts at clean `b08d1e70`. It adds an externally pinned, canonical
public policy and a separate Linux signing binary/lane, not an execution route.
Existing provisioning/controller authority and all historical contracts remain
unchanged. No key from a test or old developer build is enrolled by this slice.

Policy fixes repository 1217101761, builder hydracache-linux-observer-074-v1,
public builder key and a distinct controller/provisioning public key. The policy
digest and controller key come from independently reviewed configuration, never
from the receipt or downloaded build bundle. Rotation replaces that reviewed
pin; no fallback, automatic key discovery or old-key grace is inferred.

The protected build job runs the fixed observer commit/tree with Rust 1.94.0,
locked inputs, empty features, System allocator and no profiling overrides. It
has no signing secret. It retains native Cargo JSON, separate stderr and exact
clean Git/toolchain observations before and after the build. P0 configurations
remain separately supplied fixed bytes. Only --validate is allowed, not --run.

The separate protected signing job checks out reviewed tooling, builds the
signing executable before the secret step, downloads only this run/attempt's
build bundle, checks the external policy pin and independently supplied
controller key, verifies observation and all artifact bytes, then emits the
existing domain-separated schema-1 build receipt. It never executes the observer
or its build scripts with a signing secret. A secret is read only from a private
single-link regular file in a private owned directory, never argv or output.
Output is exclusively created, synced and never overwritten.

This is an attestation by the trusted build/signing procedure, not mathematical
proof of compilation, reproducible-build evidence or SLSA certification. The
workflow and reviewed tooling are part of the trust boundary. Environment
approval alone is not a substitute for restrictions on permitted workflow refs,
reviewed code, public pins and secret access. Private keys must not be committed
or placed on the rented performance host. That host receives public verification
material only after separate immutable-install and backend gates close.

Local tests use explicitly synthetic keys/ELF/log observations. Real enrollment
requires an operator-configured protected signing environment, fresh private
key, independently reviewed public policy digest/controller key and permitted
workflow reference. Missing configuration fails closed. No existing secret is
reused or read to create the builder policy. No workflow is dispatched by local
checks. Backend, process/tree policy, writer fencing, watchdog, reconciliation
and non-product loss rehearsal still precede any installed baseline pilot.

## Implemented boundary and enrollment procedure

`diagnostic_builder.rs` checks canonical policy bytes against an external pin and
controller key, refuses weak/shared keys and fixes repository/builder identity.
`sign_build` derives receipt fields from checked observations/content rather
than accepting an arbitrary BuildStatement. The dedicated `diagnostic_builder`
binary is Linux-only; the installed supervisor remains the default Cargo binary
and has no builder CLI/IPC/config route. The existing signed receipt format is
unchanged. Key/output parents are walked through retained O_NOFOLLOW directory
descriptors; key mode is 0600 in an owned 0700 directory. Input leaves are bounded
single-link regular files with metadata reconciliation. Publication is exclusive
and synced; an uncertain partial publication is not overwritten or repaired.

The coordinator strips runner credentials, loader variables and inherited
Rust/Cargo overrides from subprocess environments. It refuses an existing
observer target directory and checks source/input continuity through build and
four validation-only calls. Build/sign jobs use separate hosted runners, exact
checkout SHAs, pinned checkout/upload/download actions, current run/attempt
artifact names and no overwrite. The signing secret exists only in the final
signing step, after compilation/tests/download; its environment variable is
unset before invoking the signer. These checks are not OS isolation from an
administrator or a compromised GitHub build/signing platform.

Before compilation and again before signing preparation, the manual workflow
reads environment/branch-policy metadata and requires a configured human reviewer
plus exactly the feat/0.74-resp-native-throughput branch, not wildcard or tag.
This prevents an automatically created, unprotected environment from being
treated as protection. The API metadata check is not itself an approval receipt;
GitHub's environment gate and reviewed workflow/source remain trusted owners.
Relevant API schemas: [environment protection](https://docs.github.com/en/rest/deployments/environments)
and [deployment branch policies](https://docs.github.com/en/rest/deployments/branch-policies).

Real enrollment requires these explicit operator steps:

1. Select the human reviewer and configure performance-diagnostic-builder-074
   with required review and its exact branch-only deployment policy. Do not copy
   unprotected legacy environment settings or automatically approve deployment.
2. Generate a fresh Ed25519 seed outside Git and performance hosts. Register it
   only as environment secret HYDRACACHE_074_BUILDER_SIGNING_KEY_HEX. Do not reuse
   controller/provisioning seeds, test keys or print/export secrets in evidence.
3. Independently obtain the actual controller/provisioning public key; derive
   the new builder public key. Review canonical public policy bytes and SHA-256.
   Configure environment variables HYDRACACHE_074_BUILDER_PUBLIC_KEY_HEX,
   HYDRACACHE_074_CONTROLLER_PUBLIC_KEY_HEX and HYDRACACHE_074_BUILDER_POLICY_SHA256.
   No downloaded receipt/bundle is a source of these trust roots.
4. Review the exact tooling commit/workflow and dispatch the manual build-only
   lane, then manually review its protected signing job. Retain source/job and
   original artifact evidence; independently verify receipt/content and the
   external policy pin. No self-hosted runner, install or workload is involved.
5. Only after separate backend/install/loss gates close, enroll public material
   on the host and authorize the fixed baseline-only pilot. Changing the pin
   revokes the previous build-key policy for new verification; do not infer a
   grace period, cache refresh or live-policy migration from this local slice.

The initial read-only GitHub check returned 404 for the proposed new environment;
the existing performance-reference-074 returned no protection rules or branch
policy. The sealed local packet records that historical state, not a live status.

## Delegated technical review and environment enrollment

The user explicitly assigned technical review to the agent. The
[enrollment contract](diagnostic-builder-enrollment-contract.toml) maps the GitHub
required-reviewer gate to the authenticated account javaquasar (22050429), not a
fictitious AI account. An approval through this account is delegated technical
review, not an independent second-human review. Self-review prevention is false
so the authorized dispatcher can approve; the required-reviewer gate and exact
branch restriction remain. No administrator bypass or automatic approval is
authorized. Both jobs also fail closed unless the GitHub metadata explicitly
reports can_admins_bypass=false. Missing, null, numeric-zero and true values are
refused; presence of a reviewer alone is not sufficient. Each build/sign approval
must identify and review the exact tooling SHA and job evidence before releasing
that job.

The controller public key comes from a read-only extraction of verification_key_hex
in the installed /etc/hydracache-perf/supervisor-074.toml. No controller private
key, receipt, or incoming build bundle supplies this trust root. The new seed is
generated independently outside Git and sent only to the new environment secret;
public policy bytes and their digest are retained separately. Enrollment is not
host installation, artifact admission, or approval of a measured run.

The manual workflow was initially absent from main and unregistered in Actions.
GitHub requires its workflow_dispatch file on the default branch before first
manual execution; see [the documented trigger rule](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_dispatch).
Registering only that workflow on main requires separate permission and a
reviewed, verified merge path. Do not merge the entire 0.74 branch, add an
automatic push trigger, or repurpose an existing qualification workflow to work
around registration. Until registration and exact protected job approvals, no
actual trusted CI build/sign receipt exists.

The user subsequently authorized only the workflow file's default-branch
registration. A separate branch from main at 5f0259da adds the identical reviewed
workflow blob in [PR 216](https://github.com/javaquasar/hydracache/pull/216); it does
not import product or signing code. The [retained attempt](local-runs/diagnostic-builder-registration-5b75c2bc/manifest.json)
shows two red gates. CI and a local topology replay reject the extra workflow
because main's exact inventory lives in docs/testing/memory/0.71/ci-topology.json.
Full local verify also selects the existing Linux --all-features workspace
clippy command, which combines mutually exclusive allocators and is refused by
the product's compile-time guards. Neither failure permits bypassing protection,
removing an allocator guard, or treating focused Python tests as full verification.
The user then authorized CI inventory and verification tooling/tests. PR 216
now declares both jobs as manual-protected, preserves artifact budgets without
identity exemptions, binds upload/download names to SHA/run/attempt and bounds
compiler installation to ten minutes. Workspace-default lint replaces only the
invalid allocator union; all-feature lint outside HydraCache and independent
common/System/mimalloc/Linux jemalloc lanes remain. Test-first regressions
reproduced the union and missing artifact SHA, then passed after correction.
The first inventory correction exposed a second binding: memory baseline 0.71
pins that file's digest. CI's memory canary correctly refused the changed input.
The registration branch therefore restores the exact frozen snapshot and routes
current topology validation to docs/testing/ci-topology.json, matching the
separation already established in 0.74. A regression pins the unchanged snapshot
digest and the new manual-protected entry. No baseline pin is regenerated.
The 0.74 branch updates its separate live inventory, not the frozen inventory.
Full local verification and refreshed PR checks subsequently passed on c7cba271.
PR 216 merged as 2b3c5794 without bypass, registering workflow 379545618. The
[registration result](local-runs/diagnostic-builder-registration-c7cba271/manifest.json)
retains the complete successful log and preceding refusals. A single run
37923423889 uses exact reviewed tooling 33596da7; build approval is delegated
technical review through the configured account. Signing requires separate
review of the original build evidence; dispatch is not signature or admission.

The clean Linux repeat at 33596da7 passed 113 xtask library, 12 CI-reliability,
8 baseline and 78 performance-contract tests, seven coordinator Python tests,
package check/strict lint, workspace format, docs, local 0.74 contract and live
topology validation. These focused checks do not substitute for full verify.
Two full local attempts were deliberately interrupted: e3b37577 was superseded
after the memory-input canary rejected it; c7cba271 reached workspace compilation
but its debug cache threatened local disk capacity. Only the reproducible
registration target cache was removed; source and complete logs were retained.
The same c7cba271 full gate is repeated without debug symbols/incremental and
with two build jobs, as ordinary CI does. No test, threshold or frozen input
changes with that build-cache preparation, and neither interruption is a product
measurement or completed verification result. The compact repeat then refused
the missing python executable required by an API-inventory test. A task-local
alias to existing Python 3.12.3 resolved that environment precondition; the full
gate passed unchanged. Optional console verification explicitly skipped because
WSL lacks Node. This result does not assert browser coverage.

The [enrollment packet](local-runs/diagnostic-builder-enrollment-20261009/manifest.json)
records the new environment, required reviewer and exact branch, can_admins_bypass=false,
one newly registered environment secret and three public variables. The fresh
32-byte seed used the local OS cryptographic random generator; OpenSSL in local
WSL derived its Ed25519 public key from DER through stdin. Only transient process
memory and GitHub's encrypted environment secret held private bytes; no private
file, repository blob, host copy or backup was created. Public canonical policy
bytes have SHA-256 dfe81cf770088029e03f04faa4da429ef163583c0ba1b7002b8ec5606e766e22;
Rust load_policy independently checks that pin, canonical form, distinct actual
controller key and nonweak public keys. API readback verifies names, timestamps
and all public variable values, not secret contents. A real verified signature
is still required to establish the operational key/build relationship.

Technical review of the lane includes the fixed source/tree, fresh target,
credential-stripped compilation/validation, pinned actions, current-run artifact
selection, private descriptor-bound signing and exclusive publication. Hosted
runner directory permissions remain an operational preflight: the signer refuses
unsafe or aliased parents rather than loosening its checks for CI convenience.
Environment/runner administrators and the GitHub platform remain trusted;
same-account delegated review is not separation from those principals.

## Local evidence and limitations

Clean implementation `6118219fb39353ca9007f0581423e965b0e11cf9`, tree
`d78bf6b3406ec834a469626d8deb789a39fe78e0`, is repeated in
[the local packet](local-runs/diagnostic-builder-6118219f/manifest.json).
Portable policy/crypto checks, Linux descriptor permissions and a positive CLI
signature/verification round trip use synthetic ELF/log/key observations. They
prove the mechanism, not actual trusted compilation or performance. Provisioning
regression tests remain green. No lock, P0 input or qualification manifest changed.
The repeat passed 23 Windows and 31 Linux Rust checks, seven Python checks on
each OS, 90 root contract/evidence and 23 governance tests. Check/clippy/fmt
passed on both OSes; xtask check/clippy, doc/local-contract and 17 governance
CLI checks passed on Windows. Ship remained expected-red (exit 1). Full
workspace/release verification and an actual protected CI build/sign run were
not performed; no numerical release claim follows from these focused checks.

The initial test-first capture failed because the builder module did not yet
exist. Linux preparation exposed an ambiguous File::by_ref between Read/Write;
it was corrected explicitly. A coordinator fixture initially compared Windows
short and resolved long paths and now resolves its private tempfile path. Linux
rustfmt was initially absent from this exact toolchain; it was installed locally
and the clean repeat includes formatting. These are preparation failures, not
measurement attempts or threshold changes. The sealed packet retains available
raw failure logs and distinguishes observations without a raw capture.
