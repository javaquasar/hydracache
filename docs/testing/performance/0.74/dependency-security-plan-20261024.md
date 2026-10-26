# Separate dependency security gate discovered by the full milestone

## Current authority

On 2026-10-26 the operator explicitly authorized compatible security updates,
superseding the earlier audit/planning-only instruction. Keep Rust 1.88 and add
no deny.toml exceptions. Incompatible major migrations still require a separate
decision. The experimental rustls change `e0244861` was reverted under the prior
restriction; its tests, first failures and evidence remain in Git. The original
fifteen-advisory result remains historical and red, not a current database scan.
Reapplying a reviewed patch requires new exact-source gates and a distinct result;
the [earlier experiment report](rustls-security-result-20261024.md) is not reused
as evidence for a newly built release candidate.

The first newly authorized [rustls application](rustls-authorized-result-20261026.md)
is now committed separately. Only rustls and its checksum change in Cargo.lock;
focused Windows/GNU/MSRV and compatibility gates pass. The pinned database still
reports eleven findings. The remaining repairs and full milestone stay open.
The subsequent [WebPKI 0.103.16 repair](webpki-security-result-20261026.md) changes
one further locked package and passes paired public-SPKI rejection and focused
compatibility/MSRV gates. The pinned database then reports ten findings. Neither
step constitutes a full security or workspace milestone.

## First result, not a waiver

The one bounded standard `cargo xtask verify` at source `7e6c8ee8` exits 1 at
dependency policy. Source hashes and commit are unchanged; neither outer time
nor disk guard fires. Documentation, management-center, local 0.73 performance
contract, feature-leak, format, all preceding clippy lanes and dependency bans
pass. Dependency policy reports advisories FAILED, with bans/licenses/sources
OK. Later gates in that invocation do not execute. Independent earlier test
cohorts must not be assembled into a fictitious complete verify pass.

Fifteen findings cover ten resolved packages: ten vulnerabilities, one unsoundness
and four unmaintained notices. This is the policy's all-target dependency graph,
not a claim that every deployed feature/profile exposes every finding. The five
existing exceptions in deny.toml are unchanged; no new exception, target filter,
ignored warning or MSRV override is introduced. Cargo.lock remains SHA256
`d2c7736ebaa8381cece9162bbbf384db60c46111051e4e78e51d4bae12feaf5c`.

## Owners and patch/migration boundaries

| Owner | Locked package(s) | RustSec findings | Separate next step |
| --- | --- | --- | --- |
| TLS sessions, ECH, certificate decompression, key exchange | rustls 0.23.45 | [0346](https://rustsec.org/advisories/RUSTSEC-2026-0346.html), [0347](https://rustsec.org/advisories/RUSTSEC-2026-0347.html), [0348](https://rustsec.org/advisories/RUSTSEC-2026-0348.html), [0349](https://rustsec.org/advisories/RUSTSEC-2026-0349.html) | Preview 0.23.46, then real TLS/mTLS/rediss compatibility and native guards |
| Certificate path verification | rustls-webpki 0.103.15 | [0382](https://rustsec.org/advisories/RUSTSEC-2026-0382.html) | Preview 0.103.16; preserve intended rejection of distrusted intermediates |
| Bundled trust roots | webpki-root-certs / webpki-roots 1.0.8 | [0383](https://rustsec.org/advisories/RUSTSEC-2026-0383.html), [0381](https://rustsec.org/advisories/RUSTSEC-2026-0381.html) | Preview 1.0.10; distinguish the separate webpki-roots 0.26.11 shim |
| Async descriptor safety | Tokio 1.52.3 | [0386](https://rustsec.org/advisories/RUSTSEC-2026-0386.html) | Preview 1.53.3; audit the original manager alarm's one direct constructor call |
| CI/release workflow YAML | unsafe-libyaml 0.2.11 through serde_yaml | [2024-0449](https://rustsec.org/advisories/RUSTSEC-2024-0449.html) | Select a maintained parser and prove governance/topology equivalence |
| D-Bus names/wire/derives | zbus_names 4.3.4, zvariant / zvariant_derive 5.15.0 through zbus 5.19 | [0371](https://rustsec.org/advisories/RUSTSEC-2026-0371.html), [0370](https://rustsec.org/advisories/RUSTSEC-2026-0370.html), [0372](https://rustsec.org/advisories/RUSTSEC-2026-0372.html) | Explicit maintained-source and MSRV decision, not an automatic major update |
| Swagger build extraction | zip 3.0.0 through utoipa-swagger-ui 9.0.2 | [0367](https://rustsec.org/advisories/RUSTSEC-2026-0367.html), [0368](https://rustsec.org/advisories/RUSTSEC-2026-0368.html), [0369](https://rustsec.org/advisories/RUSTSEC-2026-0369.html) | Upstream/build-chain migration; its current ^3.0 constraint rejects zip 9 |

All ten inverse dependency trees are captured with --locked, --offline,
--workspace, --target all and --all-features, without compilation/execution.
YAML belongs to xtask; ZIP is the Swagger sandbox build chain; D-Bus belongs to
the supervisor/integrated tool and xtask's dev graph. TLS/Tokio have broader
runtime and test consumers. Tree membership is not an exploitability proof and
does not justify suppressing the policy.

## What the read-only previews establish

The first audit launcher fails before any resolver command because its Windows
default decoder cannot read the UTF-8 cargo log. The partial source record and
coordinator are retained. Its raw stderr was not captured to a file; the tool
transcript location and the observed exception are explicitly labelled, not
represented as a complete raw log. A distinct UTF-8 reader also extracts titles
from the actual Markdown heading, not a nonexistent TOML field.

Seven independent offline dry runs fail because the required index information
is absent. These are cache refusals, not compatibility verdicts. Separate public
registry dry runs successfully resolve rustls 0.23.46, rustls-webpki 0.103.16,
webpki-root-certs 1.0.10 and Tokio 1.53.3. The first roots argument is ambiguous;
a distinct qualified webpki-roots@1.0.8 preview resolves 1.0.10. Each command
retains its first outcome and leaves all protected manifests/lock bytes unchanged.
No update, compilation, new policy or security fix is applied by these previews.

Zbus 6 and ZIP 9 fail the current direct/transitive version requirements. The
observed public registry record for zbus 6.0.0 additionally declares Rust 1.89,
above the project's 1.88. This metadata observation is not a successful build.
The TLS/WebPKI/Tokio proposed records declare 1.71, but their complete selected
graphs still require actual 1.88 builds before acceptance. Do not use
--ignore-rust-version, silently drop a tool from MSRV coverage or rename an
unmaintained package merely to hide the advisory.

The advisory database is captured after the failed full gate at clean commit
`84d943a6deeb8cc6c6f632531aaf4f1c62275239`, with all fifteen exact Markdown
records and hashes. This is postflight identity, not a claimed preflight DB pin.

## Proposed isolated work sequence — approval required for expansion

1. Seal this first full failure, exact database records, package ownership and
   independent previews. Preserve old 0.73 artifacts and every previous result.
2. Apply narrowly selected compatible TLS/WebPKI fixes in separate logical
   commits, inspect the complete lock delta and test normal TLS, client/server
   auth, rediss/mTLS, malformed/disallowed certificate paths and both compilers.
   Record the intentional narrowing of distrusted certificate acceptance.
3. Separately update Tokio. The one direct AsyncFd::new occurrence is in
   diagnostic_manager_channel.rs::Operation::bound. The replacement registration
   needs an explicit safety argument: its privately owned cloned File remains
   open/unrebound for the entire registration, drops in the right order, and is
   not exposed to callers. Keep both deadline checks, original absolute origin,
   biased alarm selection, full owner validations and refusal behavior. Test
   FD reuse/aliases, cancellation/drop and original alarm expiry; do not merely
   silence the deprecated constructor.
4. Choose the D-Bus maintained-source/MSRV strategy explicitly. Upstream's merged
   zbus 6 wire/name modules require an API and dependency review; raising the
   project's MSRV or changing its coverage is not authorized here. Any proposed
   maintained backport needs real ownership/security support and byte/FD tests,
   not a package-name workaround. A library upgrade does not upgrade the host's
   systemd or prove ExtraFileDescriptors support.
5. Select a maintained YAML implementation. Differentially cover the existing
   workflow corpus, booleans/string keys, mappings/sequences, anchors/aliases,
   tags, duplicate/unknown keys, malformed documents and bounded input. Preserve
   CI topology, approval and release-governance accept/refuse semantics. A fork
   still using unsafe-libyaml does not remove this dependency.
6. Resolve the Swagger/ZIP build chain without disabling the shipped capability
   or trusting arbitrary download/extraction inputs. Prove contained paths,
   symlink behavior, safe permission bits, reproducible assets, licenses and
   MSRV. Adding a second zip 9 dependency leaves the vulnerable zip 3 node intact.
7. Recheck the complete graph and all affected semantics/source canaries, then
   run a distinct full milestone on the accepted changed candidate. Preserve the
   first failure above. New dependencies require a new reviewed build identity;
   no old qualification, signed receipt, performance result or root-input pin
   is rewritten or automatically promoted.

This security package is separate from the ReservedLive observation-frequency
proposal, which the operator has not authorized: preserve its previous frequency.
Neither permits weaker live checks, enlarged deadlines,
server installation/supervisor maintenance, real production enrollment, product
load or expensive qualification. The release remains unadmitted.

## Retained experiment before the explicit audit-only reply

The subsequently reverted [isolated rustls patch](rustls-security-result-20261024.md), commit
`e02448617c7073e499c1d6ddbf7d3330518db0ae`, raises its declared minimum and
locked version to 0.23.46. Windows Rust 1.94/MSRV 1.88 session-redaction/resumption
controls and focused RESP/server gates pass. The same advisory DB now reports
eleven remaining findings, not a full policy pass. This subsequent changed
candidate does not alter the first fifteen-finding result, old Cargo.lock hash,
signed inputs or rejected attempts described above. Its eleven-finding result
belongs only to that experimental dependency graph, not the restored baseline.
