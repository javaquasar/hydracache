# Signer bootstrap decision audit

Audited source is `7e4d0e3ef78cdcf3455b8719968e8e888abeeda8`. This is a
documentation and boundary audit, not a new implementation or qualification.
The [decision](../../diagnostic-signer-bootstrap-decision.md) proposes a
local-only trusted-launcher/runtime prototype and requests operator approval
before its new launch boundary is added.

At that unchanged code source, Rust 1.94 on local Linux passes the existing
eight signer executable tests. The raw capture reproduces the actual same-inode
re-exec with unchanged object/generation, and seed `0x07403b24` covers 16 fresh
process observations and four independently owned originals. Neither can prove
original invocation or runtime custody. Only owned copied-shell fixtures run;
no real credential, product process, service, cgroup or external job is used.

Documentation checking passes, and all 136 existing performance-contract tests
pass. Their captures are retained beside the Linux capture. Local admission
remains non-promotable and require-ship remains
expected red. Capture hashes and exact exits are in [manifest.json](manifest.json).
The unchanged 169-gate registry passes. The packet retains six raw captures and
six LF-normalized audited source hashes; raw capture lengths/hashes and the raw
qualification digest were checked against the manifest before commit. This
audit manifest is not a release-admission schema or independent custody receipt.
Runtime code, Cargo/workflow inputs, reviewed keys and the raw qualification
manifest remain unchanged. No lint/build or full release gate is newly claimed
for this documentation-only audit; implementation checks remain in the previous
exact-source evidence packet.

The source audit locates the fixed GNU target/command in
`diagnostic_signer_build.rs`, the diagnostic-worker-only public scope in
`diagnostic_process.rs`, and the key-reading retained producer in
`diagnostic_controller_producer_linux.rs`. Call-site inspection finds no
production producer conversion from the asserted bundle/executable guard.
The proposed launcher is absent, not silently supplied by that retained signing
API. Signing-job enrollment, worker P2/P3 and server admission stay separate.
