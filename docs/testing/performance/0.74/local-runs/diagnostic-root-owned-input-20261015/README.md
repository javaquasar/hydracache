# Root owned input paths and original reader context

The opt-in reader retains all original ancestors and the original reading
thread/user/mount/root context around positional input hashing. Its public owner
is always numeric 0/0. It is read-only inventory, not initial-host, filesystem,
runtime, job, writer-cessation or signing authority. No UID/GID or mode repair
and no actual namespace/mount modification occurs.

## Source and tests

Source `e535898005aef243316d8a9e5a81ce23acfa3d3f`, tree
`f118936fc665cd96a71883bec47ae598e2e593d7`, baseline
`f09330e3cc9c211edf6a94f814fb38c3b37e5cf6`. Preregistration `7257ae4a`
precedes code. Baseline copy integration has three passing cases; the retained
missing-module E0432 is expected red before implementation.

Final Linux suite vector: 12/12/12 new local cases, three old copy integration
cases, two new integration cases, six old issuer cases, 16 old copy cases,
18 old bundle/executable cases, six compile-fail API cases, then 12 local/two
integration cases on MSRV 1.88. Existing copy/executable regressions execute only
their disposable fixtures; no actual controller or inventoried GNU file is run.
Seed 0x0740c075 covers 64 leaf-stamp mutations, with explicit first coverage of
all 13 fields and four independent observers. Namespace/root drift positives
are real same-context observations; fault cases mutate private expected state,
not the actual host's namespaces or mounts.

Current/MSRV all-target checks and strict current lint pass on Linux/Windows.
Windows runs the six-case portable issuer suite, not Linux-only input tests.
152 source contracts, 171 registered gates, scoped format, doc-check and local
non-promotable performance contract pass. require-ship stays expected red at
the exact clean implementation. No release milestone or qualification is claimed.

## Actual read-only input observations

The local Rust 1.94 debug example is
`/home/artur/.cache/hydracache-074-194-check/debug/examples/inspect_root_owned_input`.
It is built by
`cargo +1.94.0 build -p hydracache-long-run-supervisor-074 --example inspect_root_owned_input --locked`.
Its observed hash is
`e10e5a14e2cfb6408f408cf249aa3b87f2c41c58524bc289cd841443ca2d8e8b`.
Each invocation uses `env -i HOME=/home/artur PATH=/usr/bin:/bin`, followed by
the example path and three arguments: input path, asserted SHA256 and bytes.
No key or token is accessed, and no explicit run of a selected input is requested.

One invocation per explicit file passed:

| Input under /usr/lib/x86_64-linux-gnu | Bytes | Mode | UID/GID | Links |
| --- | ---: | --- | --- | ---: |
| libc.so.6 | 2129424 | 0755 | 0/0 | 1 |
| libgcc_s.so.1 | 183024 | 0644 | 0/0 | 1 |
| ld-linux-x86-64.so.2 | 236616 | 0755 | 0/0 | 1 |

The raw capture and manifest retain each hash, file identity and before/after
metadata; all agree. Expected digests were local assertions derived by a separate
read, not selected production pins or authenticated build receipts. No ldd or
explicit execution of an inventoried input is requested; no signer/controller
start occurs. Ordinary inventory/stat/hash tools use the host runtime, whose
startup and loaded closure are not attested. The CLI's `executed=false` describes
only the absence of a requested input execution, not the tool's own runtime.
Hashing these files
does not validate ELF declarations or prove a complete loaded runtime closure.
Each example process drops its guard at exit; evidence is not live custody.

The old prepared controller at
`/home/artur/.cache/hydracache-074-prebuilt-copy-20261015.Ruo6eQvc/attempt-00000000000000000000000000000074/diagnostic_controller`
was explicitly refused. It remains 0555, single-link, 1922976 bytes and UID/GID
1000/1000 with its previous hash. The reader did not chown, chmod, copy or enroll
it. System 0644/0755 inventory does not weaken the old bundle's exact 0555 rule.

## Remaining boundary and packet verification

Current namespace/root consistency is not an independently enrolled initial
host or filesystem/administrator policy. Root owner-write remains possible;
foreign retained writers are not revoked by path observation or hashes.
Independent host/runtime/job selection and authenticated receipt acquisition,
signer credential lifetime, worker original-start, all P2/P3 writers, completion
cells 2–4 and server/qualification admission remain separate and closed.

Run `python verify_packet.py` and `python test_verify_packet.py` here. The verifier
binds 12 source files, 23 unchanged inputs and nine raw captures. All nine packet
validation/mutation cases pass on Windows and WSL Linux. It
reads repository sources/evidence and Git objects only, never actual private
keys, runtime files or binaries. `--committed <SHA>` additionally compares exact
packet bytes against Git. Inventory sizes/test counts are not performance claims.
