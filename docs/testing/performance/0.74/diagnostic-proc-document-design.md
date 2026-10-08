# Read one proc document without splicing generations

The clean `9a225347` capture failed in
`parallel_readonly_revalidation_has_no_shared_seek_offset`: one of eight readers
received `ProcessError::Invalid`. The 16 revalidation iterations and original
identity/parser checks must not be weakened or serialised to hide that result.
The failed raw capture will be retained alongside the final manager packet.

Source inspection suggests a document-assembly race: the retained proc descriptor
is read through multiple positional syscalls. Another reader may regenerate its
seq_file buffer at offset zero between the first data read and the subsequent
EOF probe. A longer newly generated document can then contribute a tail to the
already captured older document. Positional IO avoids a shared file offset, not
shared generated-document state. This is a hypothesis about the observed kernel
failure until the correction and repeat; no raw offending document was retained.

The deterministic regression will provide an old complete document on the first
read and a longer regenerated tail on a later read. The corrected bounded reader
must issue **one** positional read at offset zero with a 64-KiB-plus-one buffer,
refuse empty/over-budget/error results, and keep the original descriptor/inode,
parser, boot, process-group, generation and pidfd checks. No retry, replacement
PID, changed parser tolerance or automatic refresh is introduced. A syscall is
not general atomic-file or whole-process observation proof; subsequent original
identity revalidation remains necessary.

Tests cover the deterministic no-splice case, exact limits, empty and IO failures,
existing malformed/drift cases and the unchanged eight-by-sixteen real parallel
reader. The fcntl test must also check a nonnegative return before asserting the
CLOEXEC bit. No observer source, locks, P0 inputs, quality floor, qualification,
installed service or product workload changes belong to this correction.

The test-first capture reproduces prefix/tail assembly deterministically before
the implementation. The corrected reader uses a stack buffer of 65,537 bytes
and copies only the returned prefix into its result; it does not retain a full
buffer allocation for each small proc document. Exact-limit, over-limit, empty,
Interrupted IO and oversized metadata are fail-closed, with no partial retry.
This is control-plane correctness, not a product allocation/throughput claim.
The original Linux process group of 15 tests (including the two new tests) passed
after the correction. A clean-source cross-platform repeat is still required.

The kernel [seq_file documentation](https://docs.kernel.org/filesystems/seq_file.html)
describes stateful generation across read sessions. That supports investigation
of the multi-syscall assembly hypothesis, not identification of the exact
uncaptured kernel document that caused the earlier parallel test failure.

Clean `c2fdc3f4` passed all 15 process tests within the complete 154-test Linux
repeat; [the manager packet](local-runs/diagnostic-manager-c2fdc3f4/manifest.json)
retains the earlier Invalid failure and the deterministic test-first red log.
No concurrency, parser, identity or budget was weakened. A successful repeat
does not prove the precise uncaptured kernel cause or all possible interleavings.
