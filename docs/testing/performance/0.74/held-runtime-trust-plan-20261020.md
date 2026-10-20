# Independent held trust and original controller binding

Baseline is 792900c87d295e2895fca2da9f8d502275624cf4. Inherits docs/RULES.md.
Implement a Linux-only fixed read-only held trust source below controller_input,
reusing the unchanged trusted-input Directory/Snapshot/openat helpers. The sole
public opener accepts no path, key, UID, pin or epoch override and requires root.
It reads /etc/hydracache-perf/held-runtime-trust-074.json, not an incoming request.
No default held key/pin, config writer, production key generation or installation.

The new bounded canonical v1 config selects a distinct held builder key and
held-policy digest, worker-policy digest/epoch and frozen worker-builder-policy
digest. Controller/old builder/worker issuer selection remains the existing
reviewed constants; the held key must differ from all three. Root's protected
config is the independent operator selection, not a compiled held-key default
or a controller delegation from a v4 packet. Register this configuration in COMPAT.
Retain root, ancestors and leaf FDs; compare names, stamps and raw contents on
every revalidation. Unsafe ownership/mode, symlink/hardlink, replacement, mutation
or malformed/oversized input refuses; a failed owner cannot recover by repair.

Borrow the same SignedControllerInput in a non-cloneable/non-Send OriginalHeldTrust
guard. Observe its controller key and actual active authority policy/epoch/
revision/trust snapshot before/after verification, retaining the original request
hash. Load held policy only from retained bytes under config pins and that observed
context, with the exact reviewed old builder policy. Retain no cached policy result
in place of fresh checks. Guarded v4 inspection must also match the original
reservation request, identity, controller, worker policy/epoch, source and preset.
Refusal poisons held source and original controller/kernel/authority owners.

This is an early read-only original-controller join, not the late consuming
FixedPlacement/role-FD/full-frame join. It does not authenticate the later consent
receive, durable intent, root placement, streams, process or Armed state. No
replay slot, clock origin/expiry change, sender/manager operation, start, completion
or unlock. All 442 preceding runtime files remain after one exact module hook;
add one named runtime source, private tests and exact adapter/COMPAT guards.

Retain baseline and missing-module refusal before implementation. Test canonical
fields/key/pin/epoch refusals, actual temp-file name/metadata/content substitutions,
sticky repair refusal, seeded mutations/concurrency and the real local kernel
SignedControllerInput/authority fixture. Verify healthy revalidation and poisoned
owners for config, policy, authority and wrong-request v4 faults. Synthetic keys,
nonroot modeled roots and local kernel fixtures are not production enrollment.
Run GNU 1.94/MSRV 1.88, Windows regressions, capability docs, source/adapter guards,
strict lint/fmt, registry/docs and local non-promotable contract. Preserve all
sealed packets and qualification digest. No server/supervisor/product workload.
