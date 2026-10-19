# Consuming original manager and root guardian custody

Inherits `docs/RULES.md`. This is a local composition of the existing original
validation frame and root guardian implementation, not production host enrollment.

## Ownership boundary

The earlier borrowed `OriginalRootGuardians` refuses reuse when retired but does
not destroy its caller's still-owned manager message. Writer aliases inside that
message therefore outlive retirement. Killing the known cgroup alone cannot
establish output EOF while those aliases remain open.

`prepare_owned_root_guardians` consumes the exact `OriginalManagerMessage` instead.
Its private owner retains the entire original message/frame and derives the root
binding through the existing complete watchdog round. Existing root preparation
and enrollment execute without replacing a predicate, descriptor, timer or bound.
An uncertain preparation retains the entire consumed original owner in its error;
it does not return an apparently fresh message or permission to retry execution.

Retirement refuses the original owner before retiring controls. Declaration order
then destroys the owned message/frame before returning the root borrow. Normal
destruction cancels and reaps its collector/custodian and closes original output
aliases. Independently enrolled root guardians remain active, retaining the same
original fence/timer and revoking their exact pinned cgroup on owner/channel loss.
No completion, original-send, timer refresh or fence-release API is introduced.
The old borrowed path and its tests are preserved byte-for-byte.

## Local privileged composition

A separate opt-in root fixture retains numeric-root temporary ancestry and a real
cgroup2 root. It uses the real fixed read-only manager checks and original message,
test-signed controller/worker/OIDC inputs, and a separately executed UID/GID 65534
sender so peer identity comes from the kernel rather than an injected root claim.
Old nonroot fixture restrictions remain unchanged. A read-only executable copy
lets the nonroot sender run without traversing a private compiler cache; it is a
libtest relay, never the product executable or an installed release.

Three `/bin/cat` processes use the original streams: direct, nested and duplicated
stdout/stderr roles. Actual readiness bytes, live pidfds and real `/proc` cgroup
membership precede each fault. Acceptance requires exact SIGKILL waits, empty
subtree and drained stdout/stderr EOF, followed by an independently busy original
fence. An outside writer alias deliberately prevents EOF and can still write;
only closing that alias permits EOF. This negative is not a cgroup confinement
claim for arbitrary external descriptors.

## Creator death is a different boundary

The ignored creator victim uses a credential-checked private fixture carrier;
its schema and refusal behavior are registered in `docs/COMPAT.md`. An external
single-purpose subreaper owns the exact creator and exports its child pidfds and
pipe readers. It also holds dummy stdin writers so death of the creator cannot
produce a false pass through natural EOF exit. SIGKILL must stop the three
writers before the unchanged original deadline, not merely eventually at expiry.

On creator death the original collector/custodian must shed their output roles
and retain only fence, original timer and private socket. Their parked custody
is not completion. Both root guardians, and single loss/stall on either side,
are separate fault controls. Only exact adopted child pidfds are cleaned up;
owned writers are killed/reaped before fixture keepers. Temporary scope paths
and cgroup identities are validated independently before removal.

## Boundaries which remain closed

Test issuers, copied signed data and actual root kernel objects do not enroll a
production host or authorize a held systemd unit. The prepared original message
is not sent. The dummy executable is not the admitted original executable.
These controls cannot publish completion, release a production fence, recover
an execution capability, install a server or start a product campaign. Production
trust/pins, authenticated delivery, complete writer confinement and durable
lifecycle/reconnect still require their own admission and evidence.
