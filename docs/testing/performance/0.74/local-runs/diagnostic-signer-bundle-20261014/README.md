# Local original signer file retention evidence

The Linux fixture reader retains a directory plus six original descriptors,
streaming bounded hashes under an explicitly asserted synthetic signer
selection. It refuses equal-byte inode replacement and keeps the first observed
failure sticky. This is file-input consistency, not trusted production ancestry,
original process/runtime mapping or bootstrap authority. Exact source/baseline,
preregistration and capture/frozen hashes live in [manifest.json](manifest.json).

## Baseline and local outcomes

The baseline passed nine signer-model and 29 artifact cases before the new
Linux API target refused at its missing module (E0432, outer WSL exit 1). The
first development run passed nine cases. Added chunk-boundary coverage brings
the final suite to ten cases. An early strict Clippy pass then refused two
test-style issues, needless range-loop indexing and a generic argument borrow.
Both are retained and corrected without suppression or predicate changes.

Three final Linux repeats pass ten cases each. Seed `0x07403b23` covers 32 fresh
file mutations and four independent guards. Actual owned temporary files exercise
name/inode/root replacement, in-place and restored changes, symlink/hardlink/
FIFO/directory children, missing/extra/empty/sparse oversized inputs, owner/mode
constraints, CLOEXEC and release of seven original descriptors. Private child
checks isolate inode mismatch from simultaneous directory drift. No process is
spawned or killed by the new suite; contexts and signatures remain synthetic.

Linux regression passes nine portable signer, 29 artifact, eight reviewed-trust,
seven public API, 12 producer and 18 OIDC cases, plus 94 doctests. Windows passes
nine signer, 15 artifact and eight reviewed-trust cases. Affected package check/
strict Clippy on Rust 1.94 and MSRV 1.88 check pass on Windows/Linux. Root gates
pass six registry, 135 performance-contract and 13 release-evidence tests, xtask
check/strict Clippy, formatting, doc-check and the unchanged 169-gate registry.

The local performance contract is non-promotable; require-ship retains expected
exit 1 because exact candidate/qualification remain incomplete. No benchmark
or full release milestone is claimed. Existing locks, reviewed keys, observer
receipt/readers, producer/CLI, workflow, product matrix/thresholds and raw
qualification remain unchanged. The signer-model public verifier keeps its
schema/domain and supplied-byte semantics through one private receipt core.

## Read only verification

Run `python verify_packet.py` and `python -m unittest test_verify_packet.py` in
this directory. The verifier compares current LF source with its tested commit,
unchanged files with baseline, original observer locks with their frozen commits,
raw logs/qualification hashes and explicit false admission claims. Tests mutate
owned packet copies only. A future checkout changing a source must verify this
packet at its retained code rather than rewrite previous evidence.
Six copied-packet tests pass on Windows and Linux. Their retained first captures
cover the initial 19 build/gate logs; adding those two test captures brings the
manifest to 21 raw logs, with 11 source and 42 unchanged-file bindings.

## Remaining bootstrap

The observed handles are real; their authority is not independently enrolled.
The final directory's owner and selected key/statement are caller assertions.
Loader/runtime/bootstrap manifests are opaque bytes, and no original executable
mapping, live process, cgroup, issuer/pins, trusted ancestor or protected job is
certified. A check cannot prevent a privileged mutation after its last observation.
There is no FD/path/model export, production factory, credential reader or start
consumer. Original process/runtime custody and reviewed pre-exposure bootstrap
must precede credential coordination and original receiver/replay/fence/start,
all-writer cessation, later cells and any separately admitted server workload.
