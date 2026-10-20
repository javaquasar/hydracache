# Authority issuer key formatting control

Baseline is `b6810311b5d43c318d6c01960359e417e5d147cd`. The preceding sealed
request control records 68/184 Rust allocation requests per complete fixture
authority inspect at 1/7 generations. Only issuer-key hex formatting is a
candidate here: replace per-byte format-and-collect with the existing exact
capacity `crate::hex`. No name, inventory, context, read, metadata, binding,
failure-order, persistence or lifecycle change belongs in this candidate.

Before substitution, add seeded differential byte/canonical record/pin refusal
controls and a three-arm allocation control: frozen reference, proposed encoder,
actual `key_hex`. Use 32 warmups and three serial 1024-operation samples per arm
on GNU 1.94 and MSRV 1.88. Freeze recipes/images and retain the before series.
Also rerun the unchanged eight-case fresh leaf request control as a complete
before reference under the newly instrumented source.

Then make the one-function substitution and repeat the same controls on both
compilers. Actual formatting must match the proposed leaf's requests, and the
complete inspect reduction must equal the isolated leaf reduction at both
depths. Borrowed Boot, Namespace, Origin and all read/write request counts must
remain equal. Requested bytes include realloc sizes, not retained memory.
No latency, CPU, full ActiveAuthority or cache API improvement is claimed from
these request controls. If conservation or correctness fails, retain the failure
and revert only the candidate, not its evidence or old checks.

Seeded equivalence covers public keys, nibble coverage, canonical bytes/digests,
independent issuer/policy/epoch/host/namespace pins and refusal classification.
Existing authority journal generation, drift, unsafe ownership, sticky refusal,
policy and kernel controls remain required. Exact predecessor reconstruction
must retain every old assertion and runtime body except this one named encoder.
New manual probes are registered and normal tests remain in ordinary lib tests.

For each stage run focused tests plus affected-package all-target check and
strict all-feature clippy. Before final measurement run historical contracts,
formatting, docs, registry and local non-promotable performance checks. Freeze
source/recipe/image identities; keep before/after logs and exits separate and
never overwrite or retry a failing compiler cohort into healthy observations.
Rehash all eight prior sealed packet anchors and distinguish request evidence
from live admission. Existing live clock-order refusals remain closed.

All work is local. Preserve live checks, their frequency and 300/60/10 deadlines.
No clock or service operation, server run, installation, supervisor control,
product workload, full qualification or push through the red live gate.
