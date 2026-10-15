# Ordinary owner of a fixed original self exec

The new `diagnostic_signer_owned_probe` normal binary owns a fresh child from
fork through the first kernel exec event and the existing fixed restriction
probe's terminal result. Its owner is binary-private and thread-bound, not a
public general launcher or library guard. The normal input is only native/x32/
i386; no caller can select a command, executable, PID, FD, filter, root or key.

The parent requires an empty environment, one main thread and equal nonzero
real/effective UID/GID. It retains its own fixed `/proc/self/exe` object and
requires a read-only CLOEXEC regular single-link nonroot-owned exact-0555 ELF64
x86_64 image, 64 bytes through 128MiB. This is deliberate self-object consistency,
not authentication of independently supplied signer bytes. Standard writable
Cargo output refuses: tests run a private 0555 copy, never chmod a checkout or
build artifact. A valid 64-byte ELF header is insufficient; actual kernel exec
must succeed. GNU loader and runtime are still the current environment's inputs.

Before raw fork, argv/empty env, /dev/null stdin and a bounded nonblocking output
pipe are prepared. The child uses only fixed scalar/libc calls until execveat;
it does not allocate, unwind, take Rust locks or run destructors. It arms and
reads back SIGKILL PDEATHSIG, checks the captured original parent TGID, disables
core dumps, duplicates stdio and closes every other FD except the retained
executable before its initial SIGSTOP. NNP is set/read back before TRACEME.

The parent establishes its cleanup owner before fallible pidfd_open, waits only
for this original unreaped child and installs TRACEEXEC|EXITKILL at the exact
initial stop. Only the exact first PTRACE_EVENT_EXEC proceeds. Kernel exe metadata
is compared with the retained selected object before any new user code executes
and checked again before resume. Unknown signals/events, a later exec, object
change or phase replay irreversibly refuses. Once reaped, no ptrace/proc operation
can reuse the old numeric PID. Normal cleanup uses only the original pidfd;
pre-acquisition fallback is limited to its own still-unreaped fork result.
Before any fork, SIGCHLD must have default disposition without SA_NOCLDWAIT.
Inherited SIG_IGN/autoreap refuses instead of invalidating unreaped ownership;
the guard queries, never rewrites, the parent disposition. A child-only pre-exec
SIG_IGN negative covers the actual normal entry, with autoreap also modeled.

After resume, fixed internal child modes check the inherited PDEATHSIG setting
then enter the unchanged child restriction primitive. Direct internal invocation
without that precondition refuses. PDEATHSIG is not an authentication token:
another parent could set it too. No child mode receives authority or credentials.
Native success requires exit0 plus exactly the old fixed diagnostic line; ABI
success requires SIGSYS without core and empty output. At most one exact SIGSYS
delivery stop is forwarded for ABI probes. Output is bounded to 128 bytes and
wait/cleanup phases to five seconds. A success line is non-authorizing data, not
an independently observed installation or signed admission receipt.

Tests exercise the normal binary, not a test-only replacement. They cover all
three modes, env/argument/internal-entry refusal, writable/hardlinked images,
four independent owners and unchanged parent restrictions. Private unit tests
use only owned fixed /bin/true copies for first-exec/object/phase/drop/SIGKILL
checks: exit0 without the expected line still refuses. A missing-pidfd field
control models pre-acquisition cleanup; it is not injected kernel pidfd_open
failure. Status classification is exhaustive over 327680 fixed cases, with 256
seeded high-bit mutations. Real owned child SIGKILL/drop occurs at initial and
first-exec stops. Requesting EXITKILL and verifying PDEATHSIG are not a new proof
of parent loss at every boundary; existing dedicated loss fixtures remain distinct.

A repeat exposed ETXTBSY in the outer integration harness: parallel copy/Command
startup can let another pre-exec child retain a temporary writable copy descriptor,
even with CLOEXEC. The normal owner already closes all foreign child descriptors
before its initial barrier; that does not repair a separate outer test launcher.
The harness now serializes only private copy and Command spawn through completed
exec, not the lifetime of the four normal owners. A held-writer control requires
exact ETXTBSY without retry; after explicit writer drop a separate probe succeeds.
Kernel failures are retained, never silently retried or converted to admission.
The failed runs did not identify the actual retained writer; their attribution
to the known parallel-copy inheritance window remains an inference supported
by the held-writer control and repeated corrected runs.

The remaining joins are independently selected root/runtime and mapped-image
checks, full original-parent/creating-task loss composition, protected job/issuer
custody and authenticated signer handoff. Worker P2/P3 original-start and all-writer
revocation remain separate. No normal controller/server consumer, real key/token,
signing, completion/cells, server install or qualification is enabled. Product,
dependencies, old helpers, frozen manifest and thresholds are unchanged. See the
[preregistration](diagnostic-signer-owned-probe-plan-20261015.md) and
[contract](diagnostic-signer-owned-probe-contract.toml).
