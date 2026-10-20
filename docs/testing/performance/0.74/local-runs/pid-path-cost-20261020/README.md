# Local uncached PID pathname cost

The baseline opens a fresh process directory using a decimal String followed by
a CString. One isolated candidate derives the same signed decimal key in a
twelve-byte stack buffer on every call, then uses the existing checked opener.
It caches neither the PID pathname nor any kernel observation. This is a
diagnostic-tool allocation experiment, not product qualification.

The instrumented baseline source is
`0d7717686a06cec7178a1965e02363fc43a63da8`; candidate runtime is `b9770cc2`, and
the reviewed source is `b5f1b8fce6b3046239087873cb85ff1bfc557f0f`.
The baseline prototype's complete formatter body is moved byte-for-byte into
runtime. The new source audit compares it to the frozen prototype, and the
functional tests subsequently use the production formatter rather than a second
copy. Build receipts select one exact image per compiler/flavor and record its
source, SHA256 and recipe hash. Normal images provide fresh validation spans;
separate feature images provide opt-in Rust allocation counts.

## Allocation controls

Each arm opens the same retained proc root with the same actual PID. Both use
O_DIRECTORY, O_RDONLY, O_NOFOLLOW, O_CLOEXEC and O_NONBLOCK and perform the same
openat, proc fstatfs and close. Equal warmup is 32 calls per arm. Three
1024-iteration allocation pairs alternate AB/BA/AB; separate uncounted timing
arms use 4096 iterations. The allocation counter tracks this Rust thread, not
libc/kernel allocation, RSS or all preparation memory.

Before adoption, GNU records two requests and 14 requested bytes per open for
PID 367; MSRV records two requests and 12 bytes for PID 393. Both are three-digit
PIDs. The stack prototype records zero requests/bytes and neither arm reallocates
in all six baseline pairs. Do not turn those byte sizes into a universal PID
cost: the standard library and decimal length matter. Every raw row and the
derived summary bind their PID, digit count and measured bytes. The runtime
binding correctly refuses on both baseline images before adoption.

After adoption, PID 382 on GNU and 408 on MSRV are also three-digit keys. The
same 14/12 requested bytes and two reference requests become zero on the
production formatter in all six post-candidate pairs. Both runtime bindings
pass. Twelve allocation pairs and 72 unchanged-count fresh phase rows are
retained across the two cohorts. This accepts allocation removal, not a timing
or full execution gate.

The signed edges and 16384 seeded values test full decimal/sign/NUL content
against the standard representation, not an exhaustive enumeration of all i32
values. The buffer has room for ten magnitude digits, an optional minus and one
NUL. `unsigned_abs` handles MIN without overflow; digits and minus are nonzero,
and the untouched terminal byte is zero. The CStr view also performs its normal
checked conversion. Neither formatting nor opening introduces unsafe code.

## Preserved fresh checks

Only the two private Origin child callers change from a string key to their
original numeric credential PID. The generic string opener is retained as the
test reference; runtime has no remaining generic-string caller. Every actual
open/filesystem/object/read/projection/pidfd check retains its original order.
No new early PID admission rule, fallback, cache, clock or time origin is added.

Differential opens compare device/inode/mode and FD/status flags and preserve
missing PID, non-directory, symlink and non-proc refusals. The unchanged fresh
kernel phase probe asserts the same per-64-sender counts: 128 origin checks,
256 boot checks, 512 namespace opens, 128 pidfd checks, 128 projections and 512
nested document reads. Nested wall spans overlap; CPU is current-process CPU,
not exclusive syscall CPU. Outer wall includes trace/report preparation and
must not be used to infer I/O time from CPU subtraction.

Exact adapters reverse the named test hooks, formatter block, two calls, private
method substitution and test-only reference annotations. They do not exclude any
old validator body. Missing/duplicate fragments and unrelated live-check/time
drift refuse. The previously frozen source and all old assertions must still
reconstruct exactly. Product tree, Cargo.lock, COMPAT, existing gated registry
and qualification manifest remain unchanged.

Both compiler lanes pass 54 kernel tests (including the three PID and three
fresh-status tests), 20 held-join tests, 26 trust tests and three observer tests;
the narrower selections are not extra independent kernel counts. Strict tool
and xtask lint, targeted format, docs, registry and local performance contracts
pass. The source selection passes 350 tests: 321 performance contracts, 20 older
artifact guards, four preparation audits, two status audits and three PID audits.

## Reproduction and gate scope

Use `run.ps1 -Cohort Baseline -Stage Build -Source <sha>`, then Probe only after
all own build/test/lint jobs finish. Candidate Build additionally runs serial
kernel, PID, fresh-status, join, trust and observer regressions and strict lint.
Candidate Probe repeats the same controls. The source/contract command uses
`cargo test --locked -p xtask --test diagnostic_artifact_fixture_074 --test
held_preparation_cost_074 --test fresh_kernel_cost_074 --test pid_path_cost_074
--test performance_contract_074 -- --test-threads=4`. These source readers are
parallel, not the measurement arms. No assertions or guard schedule are changed.

Recipes refuse overwriting recorded invocations and bind source/image hashes.
Logs are PowerShell/native text captures, not exact process-byte claims. The
summary is derived from raw rows and exits; its missing, wrong and unknown
fields must refuse. The inventory rejects extra/missing/changed/reparse members
and preserves the preceding four sealed packets by rehashing their anchored
inventories. It does not rerun their source-context-bound verifiers or promote
their refused gates. Never replay into a sealed packet.
The sealed packet contains 116 hashed members plus inventory and rejects 41
missing, wrong or unknown summary mutations. Qualification and the product
tree remain unchanged; the allocation acceptance flag cannot open an execution
gate.

Allocation acceptance does not imply a latency, CPU or throughput improvement.
The full functional/native matrix, joined SIGKILL/lost-ACK and reconnect are not
rerun here; prior execution gates stay closed. No server, release installation,
supervisor/clock service operation or product workload occurs.
