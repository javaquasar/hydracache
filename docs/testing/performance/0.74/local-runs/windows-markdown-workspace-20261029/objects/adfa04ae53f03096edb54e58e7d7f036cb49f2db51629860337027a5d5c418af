# Model stage spawn preparation and retained creation observations

The [offline leaf analysis](../../docs/testing/performance/0.74/leaf-attribution-result-20261026.md)
cannot assign the spawn complement to a single API. This separate, unwired
prototype narrows the possible observation boundaries without a native control,
CLI flag, caller change, V6 replay or product candidate.

Three bracketed coordinator spans use the existing recorder mechanism:

| Span | Start | End | Interpretation |
| --- | --- | --- | --- |
| preparation | Inside original try before handle setup | Before create arguments are evaluated | Original handle/attribute/argument setup plus bookends |
| create_to_retention | Immediately before original CreateProcessW condition | After original `_child = value` in the setter | Create call, argument evaluation, WindowsChild construction and publication; not pure API cost |
| membership_release | After original owner setter returns | Before original return and its finally | Original membership/deadline/conditional resume; completeness does not prove resume |

There is no new observation between successful CreateProcessW, WindowsChild
construction and the original retained-child assignment. The first end probe
is inside the setter only after that assignment. A fatal probe there leaves
the child available through the original Job owner, including when the caller
has not yet received a child return value. The original failed-create branch
still reads last-error immediately and raises its original classified error;
no proxy DLL wrapper or post-call probe consumes or changes that error path.

The original finally releases the attribute list and inherited duplicates with
unchanged ordering/error composition. It contains no new probe. Membership,
identity, initial process validation, original conditional resume and the one
API deadline read keep their order and predicates. No wait, PID reopen,
breakaway, new containment policy or replacement cleanup is introduced.

Restricted AST controls remove only literal marks on `spawn_frames` and compare
the full API body and retained-owner setter with their originals. A fixed-index
byte control also joins the unchanged CI implementation, leaf recorder, worker
and caller to the earlier native evidence. Fake ABI differential tests compare
argv, cwd, flags, inherited handles, call order, deadlines, first refusals and
cleanup. Every added clock-read position has ordinary and fatal injected cuts.
These are models, not kernel/SIGKILL or native last-error compatibility proof.

At most six probes/eighteen reads are added per spawn: 102 recorder reads with
the existing outer and leaf layers, excluding unchanged original metric and
deadline reads. CPU is coordinator-wide, not child/native CPU. Wall brackets
are elapsed observations, not effect-only API cost. Owner leaves lie between
the retained-creation and membership spans; finally and intervening bookends
also remain outside this three-span partition. Do not add nested scopes or
promote a zero delta or arithmetic complement to causal cost.

Ordinary telemetry error freezes this recorder while the original effects and
cleanup continue. Fatal error uses the original exception/finally/owned cleanup
path. Export keeps incomplete raw records without invented ends. No caller
is connected, so integration must separately preserve operational versus
telemetry verdicts, explicit schema/budget gates, exclusive first-result custody
and the original shared 120-second/five-second origins. The prototype does not
authorize a cohort or certify recorder overhead or production execution.
