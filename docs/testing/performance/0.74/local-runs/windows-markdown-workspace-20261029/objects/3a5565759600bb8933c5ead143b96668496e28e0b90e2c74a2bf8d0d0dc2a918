# Local YAML semantic review

The Rust probe is fixture-only source. Its generated manifests and locks live
under target; there is no shipping dependency on the old parser and no new
workspace member or deny exclusion. One identical probe is compiled against
serde_yaml 0.9.34+deprecated and yaml_serde 0.10.7, with the same serde_json
version. It parses text as Value, records all typed accessors/order/tags and the
exact serialization of every subtree, and reparses the complete serialization.
It never executes a YAML run/uses field, shell, workflow, network request or
product command.

Before execution the coordinator freezes all current workflow files, fixed
valid/invalid fixtures, nesting observations and 48 generated cases with seed
0x0741028. No case is regenerated or retried after seeing a result. Every case
runs in a separate bounded child (five seconds), within the owned local
1200-second review envelope. A crash, timeout, unknown/missing/duplicate record
or changed corpus/binary refuses comparison. These review limits are separate
from unchanged product leases and package/workspace deadlines.

Acceptance requires identical parse decisions, all declared valid/invalid
expectations, exact typed values/order/tags/accessors, successful serialization
bytes and roundtrip trees. Parser diagnostics and locations are always retained;
any wording/location drift needs explicit source/consumer review and is reported
separately, never normalized out of the raw evidence. Existing CI topology and
release-governance regression targets additionally run against the real xtask
dependency before/after. The application wrappers and their predicates stay
unchanged. A probe pass alone is not a package, workspace or release admission.

Recorder failure is a failed invocation, retained unchanged. If raw child output
and the original recorded exit/timeout prefix survive, an inert recovery may
decode them into a separate UTF-8 packet, with hashes of every original file.
Recovery never infers exit status from output. A separate continuation may run
only the consecutive, previously unstarted cases, with the same frozen corpus,
binary and five-second bound. Coverage must prove no gaps or overlap. This is
explicitly multiple recorder invocations over independent per-case processes,
not a successful continuous run or a performance qualification. Unknown native
status, changed inputs or missing bytes refuse recovery; no executed case retries.
