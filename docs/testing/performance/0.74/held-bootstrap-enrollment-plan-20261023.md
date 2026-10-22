# Independent protected bootstrap selection before execution

The user approved a separately trusted bootstrap with its own enrollment/fault
contract after the sealed-memfd bind precondition failed. Do not copy the launcher,
weaken seals, change clocks/services, install artifacts or contact a server.

First implement an independent fixed root-owned configuration at
/etc/hydracache-perf/held-bootstrap-enrollment-074.json. It selects bootstrap
builder key, bootstrap policy digest and bootstrap binary digest, bound to the
same held policy and epoch. The bootstrap key must differ from controller,
worker builder/issuer and held builder. No default/generated pin or caller path
selector. Reuse retained directory/file safety primitives and borrow the same
HeldTrustInput; its old revalidation bodies remain exact. Fresh protected reads
are bracketed by the old held source revalidation; any error poisons both guards.

The canonical newline JSON schema diagnostic-bootstrap-enrollment-074-v1 has
schema_version, repository_id, bootstrap_builder_key_hex,
bootstrap_policy_sha256, bootstrap_sha256, held_policy_sha256 and policy_epoch.
Bound to 2048 bytes. Missing/duplicate/unknown fields, future versions,
noncanonical or malformed/weak/aliased keys, zero digests/epoch, changed held
policy or epoch and unsafe/replaced files refuse. Root ownership is a protected
selector, not proof a particular build/process/manager was admitted.

This first source guard must not expose a key, FD, source, start/send/release,
retry, completion or unlock handle. It does not install/enroll a server or create
the configuration. A later purpose-bound build receipt/policy and original
consuming owner join must prove the fixed bootstrap executable, then transfer
the exact sealed launcher FD and use one execveat without pathname fallback.
Actual bootstrap child credentials, full original validation, pre-exec
environment, deadline, guardian/writer revocation and uncertainty still require
their own integrated tests. The earlier activation-v1 core and 300/60/10 windows
stay byte-for-byte unchanged.

Verify decoder refusals and actual local retained file/ancestor drift; compile
and lint GNU 1.94/MSRV 1.88; enforce exact additive old-source/compatibility
restoration. No new privileged mount or original/live measurement run follows
the two retained bind diagnostic outcomes. Keep the bind gate red and document
that its normal-file partial observation was not a successful sealed-source gate.
