# Fixed production output ancestry inspection

Preregistered against clean `b76c56eeca7f59b5ba34872ed773b499be68520a`.
This W11 step adds a read-only inspector, not directory preparation or a
production execution route. The hypothesis is that a separate fixed-root
capability can enforce ownership of every ancestor without promoting a temporary
fixture's weaker ancestry assertions.

Derive only `/var/lib/hydracache-performance/diagnostics/<lease>/<surface>`
from validated startable state. Require root UID/GID zero and no special,
group-write or world-write bits for `/` through the diagnostics root. Require a
root-owned 0700 lease and a 0700 cell owned by explicitly asserted nonzero worker
UID/GID. Require the existing two distinct regular single-link 0600 streams,
that worker ownership and the unchanged 8 MiB per-stream ceiling. Numeric worker
identity is a caller assertion, not account enrollment or NSS authentication.

Retain the original full directory chain and read-only streams. Reuse the named
reader's no-follow, bounded-entry, append/high-water and sticky-refusal logic.
Expose a distinct production reader with no arbitrary path, fixture conversion,
stream access or process-binding method. Creation, ownership changes, unit
mutation, installation, workloads and qualification remain closed. This inspector
must not confer original-start authority, writer revocation or admission.

Baseline the seven existing Linux named-output tests. Test first: ownership and
mode policy matrices, invalid worker identities and expired state before any
filesystem access. Temporary fixtures exercise the shared namespace reader and
sticky replacement/shrink behavior, but cannot prove positive production ancestry.
Do not create production paths locally to obtain a positive. Run affected tests,
all-target check, strict lint, scoped formatting and root/documentation contracts;
retain raw logs and negative preparation results. No performance claim follows.

The next step remains output preparation under the host fence, authenticated
worker identity and typed composition with the original manager/process/tree and
signed executable. Sequential pathname checks are not atomic snapshots, continuous
ownership, same-open-file-description proof or durable refusal.

## Implemented boundary

`pin_production_outputs` accepts state and numeric worker identity, never a
caller-supplied root. Its private shared reader checks each ancestor before
retaining the same metadata stamp. Root and lease belong to root; only the cell
and streams belong to the asserted worker. The separate `ProductionOutputRead`
exports only refusal status and revalidation. No public conversion to the fixture
reader or process IO binding exists. The fixture entry point still refuses all
production state/install roots, and its process binding also refuses production
origin in the private shared storage.

The policy matrix checks all 4096 Unix permission/special-bit combinations for
each directory role. Owner/type negatives and invalid identity/state checks are
separate. A private wrapper seam over temporary fixture ancestry verifies sticky
failure after permissions are restored; that invented wrapper is not a positive
production observation. Existing filesystem tests still check append, concurrent
readers, replacement, symlink/FIFO/hardlink, overflow and shrink behavior.

Review removed a second metadata read of `/` between ownership validation and
stamp retention. The same validated stamp is now retained, then compared during
ordinary revalidation. This avoids accepting a different root stamp merely
because its identity remained stable afterward. It does not make the entire walk
atomic or authenticate account mapping, original start or any writer's lifetime.

## Retained local observations

The [packet](local-runs/diagnostic-production-output-20261010/manifest.json)
retains the seven-test baseline and absent-API refusal, both 150-test Linux runs
(one pre-existing ignored), three additional eleven-test repetitions, 28 portable
Windows tests and 119 root contract/evidence/governance checks. Implementation is
`df54bdca4238d12f7817ce2332ba07d75f30480b`. These are working-tree observations;
full workspace milestone verification and positive production observation remain
outside this slice. Check/lint/format, local documentation/performance contracts,
links and book build passed; require-ship refuses with observed exit 1.

The first purported baseline capture overlapped addition of test-first tests and
actually refused compilation with 20 missing API errors. It is retained as the
negative `test-first.log`, not counted as a baseline. Removing only the new tests
and repeating the unchanged seven tests produced the actual baseline. Those first
commands and the ten-test first green used the default nightly 1.98 toolchain;
final gates explicitly use 1.94.0. Nightly rustfmt was unavailable, so formatting
used the existing 1.94.0 component without installation. These behavior checks
must not be presented as a matched numerical benchmark across toolchains.

The packet's offline verifier checks retained bytes/counts and the unchanged
frozen qualification digest. It cannot certify ownership on the server, account
mapping, launch authority, physical durability or allocation/throughput gains.

## Preregistered worker traversal correction

Review after `df54bdca` identifies a launch-compatibility gap in the initial
root-owned 0700 lease policy. [Systemd 255 executor source](https://github.com/systemd/systemd/blob/v255/src/core/exec-invoke.c#L4477-L4518)
changes UID before applying WorkingDirectory. A worker without root privileges
cannot traverse that lease. The inspection tests passing therefore do not prove
that the directory layout can support the proposed nonroot unit.

Test first that root-owned 0700 ancestors/lease refuse as an execution-compatible
layout, while root-owned 0711 lease accepts. Require search permission for others
on root-owned ancestors, still no group/world write or special bits. Lease 0711
allows known-path traversal, not directory listing or namespace writes; cell
0700 and streams 0600 remain unchanged. This correction grants no filesystem
mutation or launch authority. It is a policy/source compatibility check, not a
positive systemd execution or production directory observation. Retain the first
implementation and its tests rather than rewriting them as final-policy evidence.
