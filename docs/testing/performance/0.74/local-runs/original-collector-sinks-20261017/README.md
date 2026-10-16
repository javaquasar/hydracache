# Original sink candidate withdrawn after incomplete server screen

The first sink candidate is withdrawn. Runtime, tests, COMPAT and registry are
restored to baseline `42087991f93fb1e2640e46533968ce844ddc4699`; the accepted
original pipe milestone remains active. The complete fourteen-path delta is
retained in [candidate-1.patch](candidate-1.patch), not active in the library.
Preregistrations are `e6697f0d`, `a6ab4edc` and `3caa79ad`; no deadline changes.

## What the server actually checked

The normal Rust1.94 release binary passes the BeforeCreate interruption in the
first joined chain. The next chain fails before the RootCreated checkpoint;
its assertion omitted the unexpected error. The entire matrix stops there.
The MSRV joined lane and both three-repeat SIGKILL/lost-ack matrices never run.
There are zero new sink SIGKILLs, zero lost-ack proofs and no complete joined pass.

A separate exact-runtime diagnostic adds only the attribution entry retained in
[attribution-only-test.rs.txt](attribution-only-test.rs.txt). With the same
original preparation it reports `visited=[BeforeCreate]`,
`Original(Original(Original(Causal(Time))))` and `root=false io=NotFound`.
It also checks sticky refusal, retained Busy and later fresh-context refusal.
Its exit 0 is diagnostic success with `accepted=false`, not candidate acceptance.
This proves a reachable original causal-expiry refusal before creating output;
it does not recover the error omitted by the initial matrix or quantify the
cost split between its existing checks. The challenge is never renewed.

The four kernel tests cover exclusive/concurrent creation, directory replacement,
exact metadata/status/CLOEXEC, single-link empty files, inventory and seeded
digest names (4096 values, seed `0x07405171`). Current/MSRV pass these plus two
protocol units, one sink API, old four pipe units/API, 64 selected compile-fail
examples and 63 collector/IPC/lease/fence/consent/host integration regressions.
Those local greens do not compensate for the red composed server screen.
One independent actual-kernel directory test also passes on the server.

Candidate checks pass current/MSRV Windows/Linux check/strict Clippy and 188
static contracts, registry181, scoped/full-workspace formatting, doc-check and
the local nonpromotable contract. The first registry check and missing default
nightly formatter remain retained failures; corrected canonical cfg text and
explicit Rust1.94 formatting pass without runtime changes.

The broader default-parallel `original_` local audits are separately red:
Rust1.94 records 200 passes/3 failures/7 ignores; Rust1.88 records
193 passes/10 failures/7 ignores. Their composite commands stop before later
targets; those unreached narrow targets are run separately, not substituted
for these results. The old failed source bodies are unchanged. Source identity
alone does not establish why a particular failure occurred: matched-baseline
attribution remains necessary. No broad audit was retried into acceptance.

## Setup failures and custody

The first dispatch exits before tests because its temporary root is mode 0705,
not the required 0700. Inventory shows only the two binaries and runner, no test
logs. Later escaped-name inspection identifies the empty mktemp directory with
a trailing CR and the separate transfer destination without it. PowerShell's
final CR entered the last mktemp argument; multi-source transfer then created
the separately named destination. Only that owned empty-test destination is
set to 0700 after exact owner/type/inventory checks. The unchanged guard is not
relaxed. Both unused empty CR-suffixed directories are later removed after
exact `/tmp` origin, owner, mode and emptiness checks; no evidence is deleted.

The first diagnostic upload similarly fails because its destination name omits
the CR. A corrected shell transport ends in a comment and independently verifies
the new root's 0700 mode. Its first invocation omits umask 077 and the existing
trust fixture refuses before any sink constructor. That raw setup failure is
retained; only a fresh scratch with the fixed harness umask reaches attribution.
Neither invocation is classified as a completed joined screen or a product run.

The actual joined root is `/tmp/hc074-original-sinks-20261017.L2eJxk`;
the actual diagnostic root is
`/tmp/hc074-original-sink-attribution-20261017.K5Cpie`. Both retained trees stay
private, owned by 1000:1000 with mode 0700. Signed production ancestry is not modeled by their
ownership. Raw logs are SCP copies; unsigned prebuilt test hashes and local
Git-blob observations are consistency records, not production source receipts.

Postflight confirms the original boot, installed binary/receipt hashes, active
supervisor PID8839/start time/restarts0, absent production campaign/lifecycle/
authority context and no original test executable left running. No installation,
service operation, clock change, product workload, Actions dispatch or qualification ran.

## Withdrawal and next proposal

All fourteen candidate paths are restored or absent as at baseline. Rollback
check/strict Clippy and focused original-pipe/API/compile-fail/integration gates
are recorded separately from the candidate and broad-audit observations.
Restored static187, registry180, formatting/docs/local contracts and historical
Starting/pre-consent/pipe evidence verifiers retain closed admission.

Run `powershell -NoProfile -File ./verify.ps1` from this directory. It checks
artifact/command inventories, reconstructs all fourteen candidate Git blobs
from the archived patch and original Git baseline without changing the checkout,
checks the restored paths and verifies the observed negative outcomes.

The next proposal must join Starting, pipes and sinks with constant-depth
validation and an explicit live-predicate/effect-order/deadline equivalence
proof. A successful primitive is not enough to start a worker. Original
executable/manager/process/cgroup binding, autonomous watchdog, independent
production trust, all-writer cessation and completion/cells2–4 remain open.
