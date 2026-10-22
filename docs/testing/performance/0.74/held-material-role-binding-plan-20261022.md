# Bind materialized descriptor roles under the original owner

Baseline source is 563caf77297f3e36ae660ad5706bed50d81d24dc. Materialization
already retains twenty descriptor stamps and the complete projected context,
including the original manager/client names and serial. Its validation order is
full frame, prepare, inventory, full frame, inventory. The snapshots do not yet
bind concrete inventory entries to the held launcher's inherited role numbers.

Add one private value-only role binding to those same snapshots, without a new
owner wrapper, descriptor copy, getter, message, transport or execution method.
Bind only the eight roles actually resolved by the material inventory:

| Inherited role | Source in the twenty-entry inventory |
| --- | --- |
| 0 empty stdin | 14 |
| 1 original stdout | 6 |
| 2 original stderr | 7 |
| 3 launcher release endpoint | 16 |
| 5 sealed original config copy | 17 |
| 6 original creator pidfd | 19 |
| 9 original absolute alarm | 1 |
| 10 sealed bootstrap | 18 |

Role 4 (worker) and roles 7–8 (Armed root guardians) remain under their existing
retained role/content/live validators and are **not resolved transport FDs** by
this partial map. Do not invent a descriptor getter or silently open replacement
pidfds. Exclude the creator-side release endpoint, readers, sinks, fence,
legacy/addressed aliases and custody pidfds from the inherited material map.

The pure binding uses the fresh census already produced inside each unchanged
full-frame bracket; it adds no kernel/time/bus query, removes no live predicate
and creates no new resource. Snapshot equality continues to cover every original
context field, descriptor stamp and the role map. Wrong count/order, duplicate,
negative/non-CLOEXEC entry, wrong stdio object/type/access mode or wrong new
resource shape refuses. Failure remains sticky and retires the same owner.
No origin is refreshed; the original 300/60/10 windows remain exact.

First add a red source contract, then implement and run synthetic model/kernel
controls plus strict Windows/Linux compile/lint. Native kernel tests are not a
new full original-owner attempt and cannot replace the retained first refusals.
Do not repeat the previous FD control, performance cohort or any server action.
No systemd unit, service/clock change or product load belongs to this step.

The remaining delivery path must also settle held executable addressing and the
activation environment: the current helper requires an empty environment, while
the native mechanics observation has LISTEN_PID/FDS/FDNAMES. Do not remove that
guard or enable sending from a partial map. Full original delivery, writer EOF /
revocation, durable completion, SIGKILL/lost-ACK and admission remain closed.
