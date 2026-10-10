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
