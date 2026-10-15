# Signed collector intent prerequisite

Baseline is `c34add1ba481b889c5dc46b571501d77d7b4aa5a`. The approved
[collector amendment](diagnostic-collector-design.md) requires a distinct signed
output contract before original-start can join it. The existing v1 unit appends
worker-owned regular files; neither its signature nor its digest may authorize
pipe-to-root-collector output.

Add a separate Linux byte-only projection under intent_projection. Register
collector-specific body, envelope, unit projection and signature domain v1
identities. Reuse only the unverified body field layout and externally asserted
IntentCheck; never reinterpret a v1 checked guard or retained lifecycle record.
Preserve the v1 unit builder, verifier, signatures and persisted files unchanged.

Derive a typed ordered collector unit projection from validated Reserved state.
Keep the fixed command, arguments, environment and isolation/runtime controls.
Remove worker regular-file output and writable output directories; select root
working directory, empty ReadWritePaths and zero worker file-size limit. Commit
semantic stdout/stderr pipe slots (not caller FD numbers), fixed exclusive sink
names, root-private sole writer, kernel EOF on both streams, sync, explicit
writer close, original cgroup revocation, and closure of manager/creator copies.
Keep 8 MiB per stream and 16 MiB combined. This projection is not a D-Bus unit
specification or proof that those obligations have happened.

Sign domain NUL canonical sorted JSON plus LF, with the same 4096/8192-byte and
exclusive 60-second window bounds. The separate opaque checked collector guard
exclusively borrows the original worker policy, retains exact state/controller/
peer/envelope pins and refuses drift stickily without refresh. Constructor
packet errors do not revoke a healthy worker; original worker trust failures do.
No filesystem, descriptor, process, service, journal or start API is added.

Capture baseline, missing-API red test, then implement. Test all four modeled
cells, fixed output obligations, typed command preservation, old/new digest and
signature-domain separation in both directions, canonical strictness, unknown/
duplicate fields, budgets, principal/key/window boundaries, original state/
worker/peer drift, concurrency and deterministic field mutations (seed
0x0740c097). Compile-fail tests reject cloning, deserializing, starting, promotion
or conversion from legacy checked guards. Run focused current/MSRV regression,
check and strict Clippy, format, static contracts, registry, docs and closed
local/ship admission. Retain exact-source logs and compatibility additions.

This prerequisite does not authenticate original start, kernel pipe objects,
effective environment, root collector ownership, installed policy/epoch, worker
exit, watchdog or all-writer cessation. It cannot advance Starting, completion
or later cells. Historical Kernel(Invalid) remains unresolved. No server,
workflow, product workload, qualification, push or frozen-input change is opened.
