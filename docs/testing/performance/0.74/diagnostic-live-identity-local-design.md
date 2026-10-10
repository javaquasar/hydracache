# Original diagnostic manager process and tree identity

Preregistered against clean `90aa84fcf887a4bfcb1ca4ecec9c31fee842085b`.
This W11 slice composes existing read-only observations; it does not prepare
production output directories, start or stop units, install a backend, run an
observer or change qualification admission.

The loaded invocation guard deliberately permits MainPID and lifecycle status
to change. The process reader retains one generation and pidfd; the recursive
tree reader retains one stable inventory. Checking these independently does not
establish that the manager's MainPID is the original process in the matching
cgroup node. The hypothesis is that a typed composition can reject mismatches
and propagate every observation failure without adopting replacement evidence.

The Linux entry point must borrow the original ProcessRead and TreeRead and own
the original InvocationGuard. Derive scope from validated startable state. Require
kernel tree origin, exact original boot and fixed diagnostic root path, nonzero
root device/inode, original MainPID, active/running status and success result.
Require the process's retained boot, PID and nonzero start ticks, and its exact
cgroup path to match the node containing that PID. Refuse any frozen node.
Two bounded original-manager observations bracket two process/tree checks.
Any manager, process, tree, membership or status failure permanently latches
the composite guard; restoration must not repair it. Preserve manager failure
details including helper cleanup rather than reducing them to a success.

Existing manager helper limits apply separately to each observation. Composition
introduces no retry, helper mode, wire document, syscall mutation, serialization,
signal, new pidfd or tree refresh. Local filesystem scans remain bounded by the
existing reader budgets, not a newly promised whole-operation wall-clock bound.

Tests first exercise the join and bracket ordering through private synthetic
seams: four cells, root and descendant membership, PID/status/result drift,
boot/root/origin mismatch, generation/path mismatch, frozen trees, both manager
positions, both process/tree positions, refusal after restoration and parallel
independent guards. An actual owned cat helper may verify rejection of foreign
scope and fixture trees, but cannot provide a positive diagnostic-cgroup proof.
Do not create/migrate cgroups to manufacture that positive locally.

This is a stable sequential observation, not an atomic snapshot or continuity
proof. Tree membership changes, including legitimate forks, conservatively
refuse the retained inventory. Manager ControlGroup does not authenticate the
caller-asserted root inode or original start. Output/executable/environment
binding, production ancestry, durable refusal, writer revocation, autonomous
watchdog and uncertain-intent reconciliation remain separate prerequisites.
No timing, allocation or throughput claim follows from these safety tests.
