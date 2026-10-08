# Local diagnostic spool publication

Preregistered on clean `b98be0d6`. Only Linux temporary filesystem fixtures:
no production path/route, host, service, real product build, workload or pilot.
Windows retains portable byte-verifier coverage, not Linux filesystem proof.
The observer, four P0 inputs, locks, qualification manifest and old packets stay
unchanged. The existing byte packet schema is not reinterpreted as live proof.

Inspect exactly two regular, single-link 0600 streams in a 0700 directory with
explicit fixture UID/GID. Walk absolute paths with directory descriptors and
O_NOFOLLOW; forbid production install/state roots in fixture entry points. Retain
descriptors, dev/inode/owner/mode/link/size/mtime/ctime stamps and at most 8 MiB
per stream. Compare metadata before/after reading and names/bytes on revalidation.
Oversized source files keep their observed size and bounded prefix digest only;
no tail scan, complete digest, successful packet, retry or size-limit relaxation.
Source must be stable before publication. This is not proof of stopped writers
or real cgroup cleanup; trusted fixture owner is outside the tampering model.

Publish exactly stdout.prefix, stderr.prefix and canonical manifest.json. The
manifest binds checked build identity, fixed cell, caller terminal summary,
source stamps, prefix sizes/digests and complete-versus-overflow classification.
Complete streams reuse the existing byte verifier, including retained invalid
and failed decisions; overflow has no complete inner packet. All admission,
product claims and live-process proof stay false; fixture label stays true.

An existing private output root owns a deterministic lease/surface .pending
directory created exclusively. Sync its namespace before data, create files with
O_EXCL/O_NOFOLLOW, write bounded bytes, set 0400 and sync files, set directory
0500 and sync it. Revalidate source and directory identities, then use Linux
renameat2(RENAME_NOREPLACE) and sync the parent. Never fall back to overwriting
rename, remove stale staging, promote a partial pending directory or release any
host marker. Unsupported filesystem operations fail loud with retained staging.

Explicit deterministic fault points follow pending creation, each stream,
manifest, readonly staging and final rename. Before rename, pending blocks all
later publication. After rename, exact published replay verifies every byte and
performs directory/file/parent sync barriers without rewriting or restarting
work. Conflicting published content refuses. Independent offline verification
requires an externally retained expected manifest digest plus checked build,
expected intent and terminal summary; it does not trust the manifest's decision
or hash itself. Read-only verification is structural/digest proof, not fsync.

Tests cover empty/malformed streams, bounded overflow prefixes, unsafe paths,
special files/links/ownership/modes/extra entries, retained-descriptor drift,
all six logical crash windows, conflicting/tampered/future manifests, read-only
content, external digest binding and concurrent publishers. Real syscalls on
local ext4-like WSL storage prove software barriers and fail-closed behavior,
not physical power-loss durability or production ancestry. Live backend,
watchdog, writer-revocation/host-fence integration, uncertain-ledger reconciliation
and non-product process-loss fixtures remain later prerequisites. None of this
authorizes installation, numerical admission, full D3 or release qualification.

## Implementation and preparation observations

The Linux-only module exposes explicit fixture snapshot, publication and offline
verification APIs; it has no production caller, process backend or host-marker
release. Portable receipt verification remains unchanged. Canonical manifests
are recomputed rather than accepted on their own asserted decision. Forged
admission/schema/size/unknown fields remain refused even with a matching externally
supplied digest. A synthetic valid-report fixture exercises the complete path;
its signer, build assertions, CPU numbers and terminal summary are invented.

The test-first Linux compile failed E0432 (the module did not exist) with
downstream E0282 inference errors. The first formatting check printed diffs;
scoped formatting was applied. These are preparation observations, not product
attempts. Initial seven integration tests, all-target check and strict all-feature
lint passed; the expanded exact-source repeat is retained separately. Review
also tightened end-of-read stamp checks and post-publication parent identity
validation; earlier runs must not be cited as verification of those later edits.

The first governance run refused an unregistered file-level Linux cfg target.
The test target now compiles on all OSes with its Linux tests inside a gated
module, matching the existing artifact fixture pattern; Windows runs zero spool
filesystem tests. The frozen gated-test registry is unchanged. One documentation
patch context mismatch made no documentation change.
