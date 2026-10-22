# Strict activation metadata before the unchanged held launcher

The launcher now has an explicit `--systemd-activation-v1` metadata ingress.
It accepts only the exact LISTEN_PID/FDS/FDNAMES contract, then enters the same
old core with an empty environment. Initial executable addressing is still
blocked on a separate architecture choice; no unit is built or dispatched here.

## Ingress behavior

The [baseline plan](held-activation-ingress-plan-20261022.md) pins aa58bad7.
The new entry requires the canonical decimal current PID, eight extra FDs and
exact ordered names `release:worker:config:creator:root-a:root-b:alarm:bootstrap`.
The names describe inherited slots 3–10; stdio is separate. At most four
environment entries are collected, so any excess refuses without collecting
an unbounded number of fields. Missing/duplicate/unknown/non-UTF8/noncanonical
keys or values refuse before mutation. This metadata is not authentication.

A fresh single-thread observation precedes removal of only the three validated
fields. The adapter checks that the environment is empty before entering the
unchanged core, which independently retains its old empty-environment check.
There is no env_clear, automatic detection, alternate command, legacy retry,
role-FD open/copy/remap/send or timer operation. Ordinary procfs directory
enumeration for the thread check does open a temporary observation descriptor.
The old no-argument entry still rejects activation metadata. Future entry
versions refuse. Error handling exits 2 without worker release or completion.

Original HCHL0741/HCLR0741 bytes, privilege/capability, file content, creator,
guardian, peer, descriptor, release and absolute deadline checks stay unchanged.
Current-source contracts preserve the entire old core and original material,
census, projection, consuming join, manager channel and message-owner bodies.
Only two exact hooks change the binary entry; missing/duplicated/publicized
hooks and unrelated drift fail. The new compatibility registration is additive;
all preceding COMPAT.md bytes, qualification and gated-test registry stay exact.

## Why executable addressing still needs a decision

In [systemd v257 exec-invoke.c](https://github.com/systemd/systemd/blob/v257/src/core/exec-invoke.c#L4703),
find_executable_full opens the executable before pack_fds assigns the final
activation descriptor layout at line 4759. Therefore ExecStart cannot safely
assume `/proc/self/fd/11` already names an incoming launcher. A creator-PID
pathname likewise has no admitted stable-object proof after owner loss.
Neither shortcut is implemented as a fallback.

The recommended option is a read-only bind alias of the original sealed launcher
inside the isolated attempt root. Admission must bind the alias to that exact
retained object and retain its mount through retirement and writer cessation.
This expands mount/lifecycle policy and has been requested as an explicit
architecture choice. A separately enrolled bootstrap that uses execveat would
instead add a trusted executable and its own identity/enrollment boundary.
Neither option is silently added by the metadata adapter.

The complete pre-exec environment is another unit-admission requirement.
Default manager variables must be suppressed in the admitted unit, not ignored
or removed wholesale after exec. Unknown fields deliberately refuse here;
post-exec parsing cannot prove the dynamic loader saw an admitted environment.

## Focused local checks

The first current-source contract returned the expected 101 before the ingress
hook existed. Its first output is retained. Parser tests accept every field
order and reject missing/duplicate keys, PID/count encodings, non-UTF8 values,
unknown keys and each changed FD-name byte. Three unchanged core unit tests
remain included in the six binary tests.

The real local synthetic control uses fresh owned processes with constructed
metadata and no role descriptors. Exact ingress reaches the unchanged core's
expected missing-descriptor refusal; bad count/names or an extra variable refuse
before core entry. Legacy and unknown-version entries remain refused. This is
not a healthy held launch or an actual manager activation test. No worker,
original-owner fixture or ignored fault matrix is run. A concurrent-thread
ingress fault and real manager-default suppression remain future controls.

GNU Rust 1.94 and MSRV 1.88 each pass six binary tests, one synthetic process
control, all-targets check and strict all-feature clippy. Windows passes 17
selected source contracts, check, strict clippy and formatting. Default-feature
Linux check keeps the existing allocation-control dead-code warning, without
suppression. Five offline tool controls verify fixed scope, exclusive lanes,
logger configuration and source/command/log/receipt refusal. No timing,
allocation, retained-memory or performance benefit is measured in this step.

Runtime is 795f77df2bde4fa0e11f05341e4635c7b380603a; the scope plan is 68e98cab
and fixed check tooling is eb15b745. The
[sealed evidence packet](local-runs/held-activation-ingress-20261022/inventory.json)
contains 90 members plus its inventory. The inventory SHA-256 is
`b79d75d36aa299cbb745821ce1af54c685f36f6ce92ba8af61daccdb74e0452b`.
Doc-check, all 186 gated-test registrations, local non-promotable performance
contract and diff checks pass. This focused step is not the full release gate.
All 91 packet files, including inventory.json, match their staged raw Git blobs.

The unchanged qualification digest is
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
No clocks, existing services, supervisor, installation, server or product
workload are changed. Original delivery, all-writer cessation, joined
SIGKILL/lost-ACK, completion, later cells, reconnect and server admission remain
separate unfinished gates.
