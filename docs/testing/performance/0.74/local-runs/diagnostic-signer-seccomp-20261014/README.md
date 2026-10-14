# Owned seccomp installation local evidence

Tested source: `4f8d20eeb2f630b65669da78d5551ba3bded6d50`.
Baseline: `06481accc61678025de73affe8364c43ce9f6513`.
Preregistration: `96da191195c7a7bb1a8941c76035c71e4e28486d`.
Safety amendment: `34584c4dc1bd136b62c366691af8cc6a160d1844`.
Raw captures, exact source/unchanged hashes and exits are in
[manifest.json](manifest.json). This packet checks controlled local fork-helper
mechanics, not a production signer bootstrap or credential grant.

## Exact source result

Three eight-case Linux repeats pass. Kernel syscall information identifies the
original installation operation, TSYNC flags and original header address;
bounded reads check the original length/pointer and all 272 policy bytes at
entry and after exact zero exit. The subsequent non-dumpable hardening entry
and zero exit precede the probes. Wrong flags, foreign equal-byte header/program
pointer, mutated policy and length refuse before installation continuation.

Native execve/execveat and fixed task/control calls return EPERM. Restoring
dumpability refuses. Actual x32 and i386 int80 attempts terminate with SIGSYS;
only the exact possible intermediate syscall-exit shape is accepted. Strict
info decoding, valid -1 PEEKDATA/error handling and a bounded 1024-number policy
interpreter are checked. Seed 0x07403b26 adds 16 fresh probes and four independent
tracer threads per repeat, not multiple threads in a signer process.

First-exec regressions pass 11 cases, bundle/executable 18 and producer 12.
Windows portable signer tests pass nine cases. Current Windows/Linux checks,
strict Clippy and MSRV 1.88 pass. A normal Linux library build has no seccomp
fixture symbols. Root contracts pass 138 cases, xtask check/strict lint pass,
format/docs and the unchanged 169-gate registry pass. The local performance
contract is non-promotable; require-ship stays expected red. No performance
measurement or full release gate is claimed.

## Retained failures and safety amendment

The eleven-case first-exec baseline passes. A missing implementation first
refuses compilation; the initial positive run has temporary unused-code warnings.
The first full development run passes seven cases and fails the ABI case. Two
focused investigations retain the unexpected 0x857f stop and genuine x32 exit
with value 1073741883/error false. The final observer handles exactly that exit
before requiring SIGSYS, not arbitrary stop skipping.

The piped local core collector makes RLIMIT_CORE=0 insufficient. Before final
checks, the safety amendment expands the initial 25-instruction draft to 34
instructions with only PR_SET_DUMPABLE/zero permitted. The first hardening
integration incorrectly expected its entry before the installation exit and
fails five cases; ordering is corrected to installation entry/memory/exit/memory,
then hardening entry/exit. The next eight-case development check and final
repeats pass. No seed, deadline, terminal predicate or performance threshold is
changed. One unsupported xtask registry command also fails and is retained;
the actual gated-test-check subsequently passes. No host collector configuration
or real credential is read or changed.

## Reproduction and remaining boundary

Run `python verify_packet.py` and `python -m unittest test_verify_packet.py`
inside this directory. Verification reads local files/Git only. Copied-packet
tests mutate their own temporary copies, never source or installed artifacts.
After later source changes, verify at the retained source rather than rewriting
this packet. The manifest binds nine tested-source files, 19 unchanged files
and the exact frozen qualification digest. Capture inventory and negative exit
expectations are independently constrained by the verifier.

Eight copied-packet tests pass on Windows and Linux against the initial 24
captures. Retaining their two raw logs brings the final packet to 26 captures;
read-only verification is rerun on both platforms with that final inventory.

The helper inherits the current test image through fork; first exec and exact
installation are not yet joined. TSYNC success on a single-thread child cannot
admit an already multithreaded GNU signer. No atomic proof against an external
memory writer, runtime attestation, dummy handoff or actual tracer-loss proof
is claimed. Production issuer/root/job custody, replay/common fence, worker
P2/P3, all-writer cessation, completion/later cells and server/qualification
admission remain closed. Existing first-exec source/contract, product code,
dependencies, workflows, reviewed pins and frozen numerical inputs are unchanged.
