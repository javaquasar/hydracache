# One shot request/reply mechanics, not original start

The [preregistered plan](../../diagnostic-dispatch-exchange-plan-20261018.md)
starts from c688490b; 6af23de5 records the plan before implementation. This
packet covers private message ownership and classification only. No system-bus
connection, original StartTransientUnit transmission, worker process, active
writer, fence release or completion is implemented or admitted by this slice.
No SSH, host fixture, installation, supervisor operation or product workload
runs in this experiment. All tests execute locally on Windows/WSL Ubuntu.

## What changed

DispatchExchange consumes an FD-bearing zbus message. Preparation checks its
bounded request envelope, distinct unique-name claims, body signature and two
distinct blocking CLOEXEC FIFO writers. Failed preparation retains the message.
Names are data, not authenticated PID 1/channel evidence; the body is not proof
of signed unit settings or original material ancestry. The future original join
must provide those bindings. There is no message export, ordinary send method,
connection, retry path or conversion from job observation into authority.

Arming fixes Uncertain before any transport side effect could occur. A reply
must match that request's serial, selected sender and client destination, with
exact reply shape and no FDs. Its job path must have a canonical nonzero u32 ID.
Even a valid job path is observation data, not proof that any process started.
Manager errors expose no body in diagnostics. Malformed, early, duplicate and
late replies latch refusal. A future bounded transport must explicitly close
the reply window on timeout/disconnect/error; this component does not implement
that transport or its clock. Closure cannot admit a later acknowledgement or
second attempt. Neither success nor failure proves whole-writer cessation.

The existing original message still targets the well-known systemd name and
remains deliberately unsent. It must not simply be sent or rewritten to make
these tests pass. A future trusted fixed channel must construct its unique-owner
original message inside the complete live validation frame. The old preparation,
validators, raw children, manager/systemd helpers and generic O_WRONLY watchdog
bodies stay frozen; the original O_RDWR sinks are not replaced. Original
Reserved 300 s, Starting 60 s, progress 10 s and challenge age 10 s remain exact.

## Test-first record and measured scope

`baseline-194.log` records eight existing descriptor/notice units on the previous
image. `missing-api.log` is the expected missing implementation compile failure.
The first implementation then passed four tests and failed six: its request
signature check omitted the enclosing tuple parentheses that zbus's existing
original pipe-message test already expects. That negative run is retained, not
relabelled as a pass. `initial-signature-logic.source` captures the formatted
logic before correction, not the complete byte-exact failed build input set;
`first-implementation-image.log` records the failed image hash. Intermediate
12-test compiler logs are development observations, not the final source gate.

Final GNU 1.94/MSRV 1.88 runs each pass fourteen tests. Fixed enumeration seed
0x0740d518 labels the request refusal corpus; other explicit enumerations cover
bad name claims, writer aliases/flags, reply fields/types, job-ID boundaries,
manager errors, early/duplicate/delayed responses and retained-writer EOF.
Review adds explicit oversized/extra-FD/wrong-type request and missing-CLOEXEC
cases within that same fourteen-test inventory, without changing runtime code.
Two tests transfer real SCM_RIGHTS over a UnixDatagram pair, verify the kernel's
actual local PID/euid/gid and original FD identities/flags, and compare complete
received bytes with the known-valid zbus-built message before unsafe decoding.
These are same-process socket fixtures, not a system-bus or PID 1 authentication
proof and not an untrusted production D-Bus decoder.

One socket case delivers the request then drops the peer without a reply and
explicitly closes the reply window. The exchange retains its writers and cannot
retry. Another test keeps a hidden Message clone: after a valid job observation
and destruction of the exchange, both readers still report WouldBlock; only
dropping that final alias produces EOF. This counterexample is why job receipt
or local owner destruction must never authorize fence release or completion.
It is not a process-death/active-writer revocation experiment.

Each exact release-libtest image also passes eight descriptor/notice units,
thirty-two original storage/fence units and two private compile-fail doctests.
Windows passes 44 portable units. Affected current/MSRV/Windows all-target checks,
strict all-feature Clippy, 229 source contracts, final four mutation/freeze
contracts, formatting, registry/docs and the local non-promotable performance
contract pass. The four focused canaries rerun after their source cleanup;
all 229 contracts rerun again after the test-only review. Full workspace milestone verification is not rerun for
this isolated private slice. No throughput, allocation, CPU or wall-time gain is
claimed.

The first packet verification refused its claimed eight-unit regression row:
the historical `descriptor_plan::` filter also included the newly nested
exchange tests, returning twenty-two. That invocation and the verifier refusal
are retained separately. The corrected invocation excludes `dispatch_exchange::`
from the descriptor controls and still requires all fourteen exchange tests in
their dedicated run. This corrects selection/counting, not a failing runtime
predicate or acceptance threshold. Pre-review compiler/input/contract logs are
not the final reviewed source's gate.
The previous grouped-enrollment packet also passes its read-only positive/eight-
negative verifier again; its host cohort is not rerun or promoted by these tests.

## Reproduction and evidence identity

Build each release image with explicit +1.94.0/+1.88.0, --locked and a separate
CARGO_TARGET_DIR, selecting `dispatch_exchange:: -- --nocapture --test-threads=1`.
`check-gnu.sh 194`/`188` then checks that same image before/after the regression,
doctest, check and Clippy gates. `check-windows.ps1` and `check-local.ps1` record
the corresponding local gates. These paths reflect this workstation's caches;
the scripts are not installation or qualification commands.

Seven full source capsules reconstruct the final reviewed source on 6af23de5,
through implementation 642265063c8bba31f4f71894b366f5cb53d4638e and test-only
follow-up 7fdb846aa7ebfdaed5cc866b403e131c78f56c63.
The read-only verifier requires that commit's complete tree diff to equal those
seven paths. The 329 input bindings cover all top-level package Rust sources,
root/package Cargo manifests and Cargo.lock. Registry dependency source, host
and compiler environment are not inventoried; lockfile, compiler versions and
tested image hashes are recorded separately. This is source/image attribution,
not a bit-reproducible toolchain claim. No Rust runtime source changes between
final image compilation and source commit. The static contract's final canaries
are separately rerun after their source cleanup.

`pwsh -File verify.ps1 -SelfTest` checks the closed file inventory, raw/canonical
source hashes, git reconstruction, build inputs, compiler image hashes, selected
test rows and seven negative controls without running any test process or host
operation. The manifest deliberately keeps admission, original_start,
manager_authenticated and writer_cessation false. Frozen qualification SHA256
remains 11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

Next: fixed trusted manager-channel ownership, unique-owner original message
construction joined to the complete live frame, authenticated original delivery,
autonomous active supervision and whole-writer enrollment/revocation. Only after
those proofs can active SIGKILL/completion boundaries be tested and admitted.
