# Ordinary root restriction evidence without procfs

Source `3c774c88ba99a2ea82d2d0840bdebdab97a0173f`, tree
`6af3d57900d3d0e5e05bfeb98c4fef46c42cc802`, composes the normal selected root
with the unchanged fixed restriction policy. Baseline is
`37b9b2da5df2d096921761a5e0aab80c7c5779a7`; preregistration is
`bf313f579afbff2e5aa38054a77f6549c28c6595`.

## Actual local scope

Baseline passes 37 tests: 18 root units, five ordinary child-entry, seven policy,
one root CLI and six root process tests. The new-mode red fails 0/1 against the
old binary's exit 2. Final current-toolchain tests pass 43/43 in three targeted
default-parallel groups, two after commit: 24 root units, five ordinary child,
seven policy, one root CLI and six process tests. No serial-test override or
deadline change is used. The broad regression runs separately to avoid claiming
that isolated success resolves the prior observed overlap timeouts.

Each final process group actually runs native, x32 and i386 root modes, and
installed-stop original child SIGKILL/Drop controls. Parent observation checks
native syscall order/returns, real stack header/BPF, TSYNC, successful ND0 set,
kernel filter increment and retained context. Native requires its existing 20 EPERM
checks and exit 0; alternate ABI requires coreless SIGSYS. Every accepted path
requires original reap/ECHILD, pipe EOF and exact root cleanup. The narrower
root-exec and four further independent sequential owners still pass.

The six new units cover syscall decoding, filter state, dirents, event ordering,
retained reader provenance/flags and actual original-child memory reads. Test-only
POKEDATA changes a private pre-exec stack to exercise 256 seeded BPF mutations,
wrong headers and memory bounds; this is not a loaded-runtime or signing proof.
Normal observation never writes child memory. Missing EOF preserves the root.

MSRV 1.88 passes 43 tests. Its normal binary has an extra libm and explicitly refuses
all five new modes, preserving the fixed four-role graph; it is not admitted runtime
support. The ordinary entry's non-rooted actual kernel denial tests do pass on MSRV.
Private test binary copies only are stripped. No dependency, compiler/profile,
policy-byte, qualification-input or product-code change is used to gain acceptance.

Separate old regressions pass 121 tests: rooted/loss 83, first-exec 11, private
isolation parsers 3, old owned-probe units 12, GNU 6, owned CLI 1 and processes 5.
Static contracts pass 161; the gated registry remains 175. Windows current/MSRV
portable groups pass 8 each. Windows/Linux package checks and strict Clippy pass,
as do MSRV checks, formatting, doc-check and local performance-contract. Require-ship
refuses with expected exit 1; no full release gate or server qualification is claimed.

## Negative feasibility and tooling observations

The attempted post-install PR_GET_DUMPABLE readback is rejected by the exact
existing prctl policy. The instrumented capture observes TSYNC returning 0 and
SET_DUMPABLE(0) returning 0, then GET_DUMPABLE returning -1/error. The new readback
was removed; no policy exception was added. Actual ND0 set success, exact BPF,
filter increment and restoration denial are the accepted evidence instead.
The original preregistration remains in Git; its feasibility correction is explicit.

The first development refusal predates instrumentation. Another capture reaches
successful native completion but the test correctly rejects verbose development
stderr. Instrumentation is absent from final source. Clippy initially encounters
an internal diagnostic-rendering panic; that capture is tool-truncated and marked
incomplete, not a full compiler trace. A short-format retry exposes two real lint
errors (alignment idiom and redundant cast). Both are corrected, and strict Clippy
then passes with its ordinary output format. No lint suppression/toolchain change.

The earlier root-exec packet's two overlapping-broad-load process timeouts remain
byte-pinned against the baseline Git object. They are not fixed by this composition
step or silently discarded. Five-second waits and outer guard remain unchanged;
root retention after owner SIGKILL and load robustness remain separate risks.

## Integrity and reproduction

The manifest pins 16 implementation files, 13 unchanged policy/old-fixture sources,
both frozen inputs and 20 captures. Captures use LF and one trailing newline. These
are unsigned development observations, not clean-checkout execution attestation.
The verifier checks exact summaries/markers, negative results, the incomplete ICE
label, source tree/Git objects and contract bounds. It also mechanically compares
the old ordinary entry's proc pre/post checks and native exercise with the extracted
source, rather than inferring preservation solely from a passing model.

Run `python -B verify_packet.py` and `python -B -m unittest discover -s . -p
test_verify_packet.py` here. Eleven verifier tests reject changed pins/claims/bounds,
capture tampering, missing evidence and relabelled refusal. Historical Git objects
are checked without requiring a future checkout to keep this source unchanged.

## Remaining work

Full ordinary-entry creator loss before/after installation and protected one-shot
handoff are next; these new controls kill the child, not its creator. Production
runtime custody, real credentials, authenticated worker start, all-writer revocation,
completion/cells 2–4, server rehearsal, qualification and ship admission stay closed.
No remote operation, signing, product workload or performance claim belongs here.
