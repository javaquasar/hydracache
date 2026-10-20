# Fixed slice kernel identity checks in a private namespace

The [local plan](fixed-slice-namespace-plan-20261020.md) tests the existing
[fixed slice observer](fixed-slice-design.md) against actual cgroup2 directories.
It separates a successful kernel identity check from systemd ownership and
original attempt admission. Those latter gates require different producers.

## Temporary topology and unchanged observer

The root harness reserves a unique ordinary temporary name and creates its exact
empty cgroup subtree below the local WSL mount. Synthetic system.slice and
other.slice parents, each with the derived attempt child, live only inside this
subtree. The harness never moves a process into it and never writes cgroup
controls. It remains outside the fixture root throughout.

Every case execs a fresh copy of the selected test image. Only its test thread
unshares a mount namespace, makes mount propagation private and bind mounts the
temporary subtree at that namespace's /sys/fs/cgroup. The unchanged PinnedRoot
and FixedSlice implementations then observe actual cgroup2 objects at their
fixed paths. No path override, fake Stamp or alternate acceptance implementation
is injected into the runtime observer.

Every case first requires a successful empty-root pin, fixed slice pin and
revalidation, plus actual read-only and CLOEXEC observer descriptors. Separate
cases then exercise a different attempt, replaced named root, replaced parent,
unsafe root/parent modes, replaced mount anchor, original refusal and an
identically named root below another parent. Healthy original revalidation
remains possible after merely presenting a different attempt or foreign root;
the higher-level original holder still supplies its own sticky refusal policy.

The parent reaps the child before removing its exact empty fixture directories,
including the known replacement names. There is no recursive deletion. After
each case its own namespace, mount anchors and actual host system.slice stamp
must equal their initial observations. The child namespace disappears with the
child; no host mount is replaced. The twenty-second wait is a test-harness bound,
not an admission lease or a change to an original deadline.

## Test thread identity

The first selected-source GNU and MSRV runs refuse before the bind mount. Their
unshare call succeeds on the Rust test thread, but /proc/self/ns/mnt reports the
thread-group leader's unchanged namespace. Comparing that symlink is not a
measurement of the thread that will perform the mount.

The isolated correction uses /proc/thread-self/ns/mnt for all four new fixture
observations. It retains the pre-mount inequality, parent namespace equality,
host anchor checks, complete positive-pin prerequisites and all refusal cases.
Neither failed cohort is a positive pin. A source-audit invocation interrupted
before the correction is also retained as incomplete, not a passing gate.

The next GNU/MSRV cohorts pass their actual healthy pin and different-attempt
controls, then the kernel refuses the fixture's cgroup rename with EPERM.
That refusal says nothing about the observer's inode-substitution predicate.
The final mutation removes only an exact empty fixture cgroup and recreates its
name while retaining the original descriptors. Parent replacement first removes
its empty child and then requires the recreated parent to have a different inode.
No case is skipped and no expected refusal is replaced with an earlier error.

The 321-contract image is compiled and started on 75f3aeab across the later
fixture-only HEAD advance. Its dynamic evidence report reads the current
HEAD/worktree; this is not a stable-HEAD qualification gate or a fresh run of
the final kernel image. The subsequent change is confined to the external
fixture and its plan; every input under the old supervisor source/xtask contract
directories, registry, COMPAT and Cargo.lock remains identical. Six exact source
guards are repeated on the final selected source independently.

## Boundaries before actual delivery

A private namespace with synthetic system.slice is not a manager-owned attempt
slice and cannot enter the original production validation frame. This fixture
supplies no guardian Armed reply, original controller/manager join, authority
epoch, barrier release, writer-cessation receipt, completion or fence release.
All original live checks and 300/60/10-second origins and bounds remain unchanged.

The existing BuildStatement, checked builder policy and fixed nine-file bundle
authenticate only the frozen worker recipe. They cannot be reused to certify
the v2 launcher/watchdog/collector hashes: those executable roles have no matching
builder receipt or installed-object producer yet. The old direct-worker unit
therefore remains unsent. Controlled manager ownership/exclusive population,
role-specific trusted artifacts and the complete held-unit projection still
precede original delivery and its new SIGKILL/lost-ACK matrix. No server,
installation, supervisor operation or product workload belongs to this fixture.

## Completed local kernel gate

Selected source 2f27453f passes all nine kernel cases once on GNU 1.94 and once
on MSRV 1.88. Each compiler also passes the nine existing ordinary controls,
247 docs and strict all-target lint. Every case reports positive pin before its
mutation, read-only/CLOEXEC observation, unchanged host anchors and cleanup.
The local clock service is active and unchanged.

Six exact source guards, Windows sixty unit/twenty doc tests, strict lint,
formatting, registry/docs and the non-promotable local contract pass on the same
selected source. The separate reference contract image passes 321 controls,
including closed ship admission, across the documented fixture-only HEAD advance;
it is not a stable-source qualification run. The
[packet](local-runs/fixed-slice-namespace-20261020/README.md) retains exact compiler
images, recipes and all preliminary failures. This closes the private observer
kernel gate only. It changes no product runtime and establishes no speedup or
actual manager-owned original attempt admission.

The immutable packet contains 95 hashed files plus its inventory, SHA256
00e1dc901bf374c3f49dee27bb466ef709796b39010878e992fe27a97d08e411.
The verifier rejects 27 metadata mutations without editing retained files.
