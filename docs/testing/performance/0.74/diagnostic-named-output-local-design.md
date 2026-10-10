# Fixed named output observations, local fixtures only

Preregistered against clean `5f1f4edc8398900d3eed7f030dbf941a7db77c36`.
Hypothesis: retain directory descriptors and compare fixed named streams around
the existing original-process IO reader to reject namespace substitution that
inode-only observation deliberately accepts. This is a correctness hypothesis,
not an allocation, throughput or installed observer execution claim.

Derive `<fixture-root>/<lease-id>/<surface>/stdout.json` and `stderr.log` from
validated startable diagnostic state. Refuse production state/install roots,
relative, noncanonical, overlong and overdeep paths. Walk with retained read-only
directory descriptors and O_NOFOLLOW, pin every observed directory identity and
ownership/mode, and require explicit fixture UID/GID and mode 0700 at the fixture
root, lease and cell. Earlier ancestors are identity observations, not a trusted
production ownership policy. Require exactly two distinct single-link regular
0600 streams, read-only descriptors and the existing 8-MiB ceiling per stream.

Revalidation checks every retained parent/child binding and both fixed stream
names. Allow append timestamps and length growth; refuse observed shrink,
overflow, namespace, ownership, mode or link drift. Never read stream content or
change shared seek. Failure permanently refuses the object, including after
repair. Bracket original-process IO checks with named-output checks in a typed
fixture-only wrapper; do not export raw stream descriptors. Match exact state
with checked start material before binding, retaining explicit fixture origin.

Test first against the absent API; retain baseline and negative preparation
results. Cover append, concurrency, replacement of stream/cell/lease/ancestor,
repair after refusal, extra/missing entries, symlinks, hardlinks, special files,
ownership/mode, invalid state/path, shrink/overflow and original-process binding.
Use only temporary Linux storage and owned non-product helpers. Windows has no
Linux named-output proof. Check affected tests, check/clippy, scoped formatting,
root contracts, documentation and local performance-contract gates.

This slice creates no production directories and runs no service, observer,
host operation, product workload or qualification. Production output preparation
still needs reviewed ownership/ancestry, host fence and lifecycle integration.
Sequential observations are not atomic namespace/FD snapshots, continuous
execution, append-only content, writer revocation, durable failure or start
authorization. Manager/process/tree composition and backend enrollment remain
closed. Frozen observer/P0/locks/policy/qualification inputs stay unchanged.

## Implemented boundary

`FixtureOutputRead` retains the full root-to-cell directory chain and private
read-only streams. Each revalidation compares retained and freshly opened
parent/child identities, checks exactly two cell entries and advances a per-file
length high-water mark. Directory modification times and link counts are not
frozen: unrelated sibling creation must not reject the original path. Owner,
mode, device and inode remain pinned. O_NONBLOCK prevents a substituted FIFO
from blocking the read-only open before its nonregular metadata is refused.

`FixtureProcessIoRead` borrows that original output guard and the original
process reader. Named checks bracket existing IO revalidation, without exporting
stream FDs. Binding requires the exact original state and fixture start material;
checked material revalidation still supplies the private signed executable.
Named or process failure refuses the named capability as well as the composed
reader, including after the wrapper is dropped. It does not write a durable
failure journal, continually revalidate the installed bundle, compare a live
manager invocation or prove the process was launched from that state.

The composition regression leaves an owned cat copy writing into its original
single-linked inode, moves that file outside the cell and creates a new
`stdout.json`. The earlier inode-only reader still accepts, while the new
composed reader refuses and remains refused after restoration. Synthetic policy,
signature and Cargo log retain their earlier fixture-only meaning.

Do not reuse the stable terminal spool snapshot for this live append observation:
its exact timestamps/content checks intentionally reject changing bytes. The
new guard observes names and bounded metadata, never content. It therefore
does not detect all in-place changes or replacement-and-return between reads.
Temporary ancestry, explicit UID/GID and successful local checks cannot certify
the fixed production root, start authority, stopped writers or release readiness.

## Local evidence and preparation refusals

The [retained packet](local-runs/diagnostic-named-output-20261010/manifest.json)
records the 25-test process/IO baseline at preregistration-only HEAD `2eabe6a3`
(code unchanged from `5f1f4edc`), absent-API compile refusal and pinned compiler
diagnostic-renderer panic. The first static root guard also refused a rustfmt
line break between flags; whitespace normalization corrected the guard without
changing runtime checks. The initial root target's 80 passes/one failure remain
retained alongside the successful repeat, not discarded as noise.

Final checks passed 210 Linux tests, with one pre-existing ignored, 28 portable
Windows tests and 117 root contract/evidence/governance tests. A post-review
seven-test run and three complete ten-test repetitions additionally verify the
seek-position assertion and composed binding. Windows executes zero Linux
named-output tests. Scoped formatting, affected all-target check/all-feature
lint, xtask lint, documentation/local performance contracts, links and mdbook
passed. Full workspace milestone verification was not rerun for this slice.

Implementation is `5156c636aa66a0e881b8227c386081e6a338e231`. Captures are
working-tree diagnostics before that commit, with final documentation checks
afterward, not clean-source qualification receipts. Stream content, installed
execution, production preparation and numerical admission remain unproven.
The offline packet guard checks retained byte lengths/hashes and counts; frozen
qualification manifest digest remains unchanged.
