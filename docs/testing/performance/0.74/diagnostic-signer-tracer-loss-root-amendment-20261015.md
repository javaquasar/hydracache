# Exact temporary root ancestry and refusal attribution

The first ten-case implementation passes three tests and fails seven positive/
control cases. A focused capture locates `directories: owned runtime root refused`
before any admitted tracer kill. This is not a failed EXITKILL observation.

The existing Root::materialize creates `<TMPDIR>/.tmpName/root`, not
`<TMPDIR>/.tmpName`. Its original four descriptors name `root`, `root/lib`,
`root/lib/x86_64-linux-gnu` and `root/lib64`. Preserve that materializer and all
old helper/profile/policy bytes. The observer must require exactly that two-level
shape below its own original temporary parent: an ASCII alphanumeric temporary
container name of at most64 bytes, followed by literal `root`. Open the container
relative to the original parent, then literal root/lib/architecture/lib64 relative
to the preceding original open directories, all with DIRECTORY/NOFOLLOW/CLOEXEC.
The container remains its existing owner-only0700; all four selected directories
must still match the report's original kernel inode identities and0555 readonly
FDs. Recheck the original parent around this sequence. No arbitrary descendant,
symlink, alternate root, pathname-only trust or changed tree is accepted.

The first generic negative assertions also passed under that unrelated directory
failure. They do not yet establish each intended refusal. Require wrong-stage at
frame decoding, short/missing-FD at report parsing, self-pidfd at kernel child
binding, and wrong-directory at directory binding. Retain the earlier coarse
results as development failures, not final evidence of those five properties.

Mark the inherited report FD CLOEXEC immediately in the dedicated tracer role
before invoking setup utilities; only the original child dummy channel may
survive its own fixed exec. No protocol size, timeout, syscall policy, toolchain,
qualification digest or production admission change follows from this correction.
