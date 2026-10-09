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
