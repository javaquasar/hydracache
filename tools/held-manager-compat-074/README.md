# Local held manager compatibility refusal

This is a separate compatibility precondition, not original delivery, an
all-writer terminal proof or production admission. No original owner, full
validation frame, envelope, release packet or writer FD is accepted or returned.
Existing runtime checks and the original 300/60/10 windows are unchanged.

Offline synthetic controls, without any bus/process launch:

```text
python -B tools/held-manager-compat-074/test_probe.py
```

The explicit `probe.py --attempt <fresh>` command is restricted to root in the
exact local Ubuntu WSL repository. It requires the observed systemd 255 manager
and rejects a missing/nonunique/non-PID-1/non-root manager. Each query uses a
unique destination pinned before the one possible mutation. These separate
busctl connections are not the retained original manager channel.

The fixed probe submits one StartTransientUnit request on a system-random
hc074-fdcompat-<32 hex>.service name proven absent beforehand, mode fail, with
an empty ExtraFileDescriptors array and a fixed /usr/bin/false helper. No Hydra
worker or other command can be selected. It expects the exact unsupported
property rejection and proves the unit absent afterward, then rechecks manager
and local context. There is no retry, fallback, stop/reset or existing-service
mutation route. An uncertain/present unit stops the test and retains its exact
name for inspection; no blind cleanup may affect another unit. All calls retain
raw stdout/stderr, argv, outcome and first failure in a non-overwriting folder.

Per-call bus timeout is 250 ms, process guard at most two seconds, and the
whole compatibility test has ten seconds. These bound this separate test only;
no original reservation, controller or cell timer is created, renewed or changed.
Unknown/malformed versions and non-property errors cannot become a supported
transport result. Even a newer version needs a separate actual FD mapping proof.

The official [v257 implementation](https://github.com/systemd/systemd/blob/v257/src/core/dbus-service.c)
accepts the extra FD property; [v255](https://github.com/systemd/systemd/blob/v255/src/core/dbus-service.c)
does not. This does not prove that updating systemd alone completes held launch.
Keep current services/systemd untouched. A new FD bootstrap/broker protocol or
another approved test environment is a separate architectural/operator decision.

The first v1 tool invocation stopped on a read-only GetNameOwner JSON decoder
error, before any StartTransientUnit. Its source/receipt remains immutable.
The [v2 correction](../../docs/testing/performance/0.74/held-manager-compat-plan-v2-20261022.md)
requires busctl call's exact single-argument arrays and adds the captured reply
regression. It retains the same checks and budgets; the fresh v2 control is not
a retry of a failed mutation or a new original admission.

V2 then stopped on a read-only property-interface typo, also before mutation.
The [v3 correction](../../docs/testing/performance/0.74/held-manager-compat-plan-v3-20261022.md)
retains its red exact-argv regression and all earlier sources. Eleven final
offline controls pass per platform. The [actual result](../../docs/testing/performance/0.74/held-manager-compat-20261022.md)
has exactly one StartTransientUnit, the expected unsupported-property rejection
and proved absence of its unique unit afterward. Original delivery and all-writer
cessation remain false; do not rerun this lane or substitute a transport.
