# Root transport EOF scope follow-up

Candidate `079ee239` closes the remaining immediate-EOF oracle scope defect in
the root transport module. Both first changed GNU/MSRV focused runs pass six
cases; one unchanged manual cgroup case remains ignored and 1616 unrelated
library cases are filtered by the explicit module selector. Neither run is a
full ordinary suite or an original-owner attempt.

The predecessor's complete MSRV v9 suite retained an EOF refusal in
`cgroup_root_transport_size_flags_missing_credentials_and_fake_roots_refuse`,
reading -1 instead of zero at the same unchanged helper. Its GNU counterpart
passed. The [preregistered successor](transport-eof-scope-plan-20261024.md)
audits all three calling functions, rather than treating one later pass as
proof of the historical holder's identity.

The new delta wraps only the other two EOF callers in the already checked
owned-case scope. It preserves every old test body, actual EOF assertion,
credential/rights/flag/size rejection, deadline and parent parallelism. The
helper does not change. Scope cleanup, timeout or surviving descendants still
cannot count as a pass. No runtime transport or production writer proof changes.

Both compilers pass all-target check and all-feature strict Clippy with unchanged
source maps. Twenty current/history/mutation controls and xtask check/strict
Clippy pass. The distinct complete source cohort also passes all 321 tests,
zero ignored/filtered. Its unpaired elapsed time is not a performance claim.

The [49-member packet](local-runs/transport-eof-scope-20261024/index.json) has
39 objects and SHA256
`3f5885dd8b2692ed79ecb4b5d0a085b8a9e0485dbf03406f183d3cd69c7dd6d0`.
Its four changed compiled input files are retained directly. The other 707
inputs reference exact verified objects in the predecessor's 5d32f67e... packet;
member names, sizes, bytes and source-map identities match. No ELF is published.

The old complete GNU/MSRV v9 failures remain first results. A complete kernel/
time cohort is not repeated to find a green interval. This successor proves its
focused scope/closure controls, not current full Linux green status, all-writer
retirement, release admission or successful preparation inside the original
window. Server, clocks, services, enrollment and product load are untouched.
