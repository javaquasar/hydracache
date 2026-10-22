# Native FD CLI correction — separately identified attempt

Source 4befa365's first control fails locally with `busctl: unrecognized option
'--helper'`. The StartTransientUnit-shaped CLI argv is recorded, but getopt
rejects the payload option before bus connection/message delivery. The random
service was absent beforehand; cleanup's property lookup only returns the
default name Description, not the intended marker, so no StopUnit is submitted.
Its first receipt, empty helper files and all command outputs remain unchanged.
This is not a failed original delivery that may be retried.

An offline regression first fails on the missing option terminator. Fix only
the busctl invocation/recorded argv to put `--` before payload arguments. Use
new evidence `target/held-native-fd-v2-20261022` and a fresh random name, never
reuse/resume the previous control. Same fixed descriptors, predicates, 250 ms
per-call bounds, helper start/stop bounds and no original reservation. Only one
submission in this corrected version; retain its first outcome regardless of
success. No retry after a dispatched error/lost reply. All scope/limitations of
the [initial plan](held-native-fd-plan-20261022.md) remain in force.
