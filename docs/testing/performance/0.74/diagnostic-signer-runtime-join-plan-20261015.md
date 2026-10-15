# Original GNU runtime and receipt consistency join

Baseline is `fca98a632d0917094fd4680ba3d19816a3c0defd`. The operator requested
the next local stage. Join the existing numeric-root original-file reader,
shared GNU declarations and separate signer-build signature verifier without
promoting any of their explicitly asserted selections to production authority.

The hypothesis is that the current opaque runtime-manifest gap can be closed:
the manifest checked by the receipt must be canonical JSON plus LF generated
from the same four retained original files, and the receipt's binary must be
read from the signer original, not supplied separately. Three other build inputs
(lock, build log, bootstrap context) remain explicit supplied bytes. Expected
statement, issuer, paths and four digests must be selected before acquisition;
never derive them from the incoming receipt or catalog. This proves consistency
under the supplied issuer, not independent enrollment or build provenance.

Add a Linux normal-library join owning all four original readers. Public
acquisition uses only their existing fixed numeric-root/current-root constructor;
no owner/anchor/FD/PID override, fixture/copy conversion or repair. Add only a
crate-private bounded positional byte-read seam to that reader. Check original
context/path/metadata/content before and after reading; exact bytes/hash and EOF
must match. All four 64-byte..8 MiB budgets precede file acquisition. Verify the
existing exact expected statement and signature before acquisition, then verify
all receipt content against the actual signer and generated runtime catalog.
The old receipt format/domain and old opaque verifier remain compatible.

Retain original descriptors; subsequent checks cannot adopt replacement files.
Any read/join/recheck error permanently refuses the owner and every retained
input. No Clone, Deserialize, FD export, thread transfer, bootstrap/start/signing
conversion or authority field is added. Numeric root, mode bits and sequential
rehashing do not revoke retained writers or authenticate initial-host/filesystem/
launcher/job custody; kernel/root and ACL/mount administration stay outside this
observation. No actual private key, receipt signing, controller execution, token,
GitHub/server mutation, product workload or qualification is authorized here.

Capture existing build/GNU/root-reader baselines before mutation and a missing
join API failure. Test synthetic signatures and independent synthetic GNU images
in owned temporary roots through a cfg(test)-only reader seam. Public acquisition
must reject that nonroot fixture. Cover canonical generated manifest vs opaque/
drifted manifest, wrong key/signature/domain, role/size/hash/companion mismatch,
equal-byte replacement of every role, sticky failure after restoration, ancestor
drift, positional offsets/bounds and four independent thread-local owners. Seed
`0x0740c077` covers 64 deterministic content substitutions. Positive fixtures
must remain labeled synthetic; do not sign an actual receipt just to create one.

Run repeated focused Linux tests, portable old integration tests, compile-fail
constraints, current strict all-target lint/check, MSRV 1.88, root contracts,
scoped formatting, documentation and local non-promotable performance checks.
Retain failures and exact source/evidence commits separately. Frozen lock,
qualification inputs, reviewed pins, workflows and product code stay unchanged.
Document the new optional use of canonical catalog bytes in the old receipt's
runtime-manifest slot; it is not a new config reader or authority enrollment.

This joins observations, not the production launcher. Actual independently
admitted runtime/root/receipt and protected signing job, original-start and all
P2/P3 writers still precede completion, cells 2–4 and server rehearsal.
