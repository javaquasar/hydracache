# Ordinary selected root restriction composition

Base `37b9b2da5df2d096921761a5e0aab80c7c5779a7`. Evaluate the existing fixed
restriction policy in the ordinary selected proc-less root, without adding a
procfs mount, inherited host proc descriptors in the child, caller-selected inputs,
new public mutating library API, credentials or workload. The previous root-exec
packet and its overlapping-load timeouts remain historical evidence.

## Hypothesis and mechanism

Extract the fixed syscall-mutating installer and existing native/x32/i386 denial
exercises into one private source used by the existing isolation entry and the
root probe. Keep the exact 64-instruction, 512-byte policy and TSYNC flag. The old
entry retains all procfs pre/post checks; the new root entry uses its retained
original parent to check context instead. Add a direct dumpability readback.

The parent must observe actual native syscall entry/exit after the loaded stop,
validate the stack-bounded sock_fprog and full policy bytes at seccomp entry,
require successful TSYNC, actual no-dumpability installation/readback, and exactly
one new filter in kernel status. It must reject unexpected events, order, argument
shapes, memory bounds and installation failure, not trust a child readiness marker.
Before no-dumpability, retain the child's original environment and FD directory
in the PARENT only; after it, use bounded retained readers without fresh-open
fallback. Track the fixed install sequence so descriptor mutations cannot pass.

Separate fixed root-native/root-x32/root-i386 modes resume from the validated
installed stop only to the existing nonproduct denial exercises: native EPERM
checks end in exit0 plus fixed data, alternate ABI ends in coreless SIGSYS. All
require original reap/ECHILD and pipe EOF before exact root cleanup. Existing
root-exec still kills at its loaded stop without installing policy.

## Frozen bounds and falsification

Retain four images, 8 MiB/image, maps 65536 bytes/128 lines, five-second event
wait and five-second process outer guard. New install observation allows at most
128 syscall events and reads at most 512 policy bytes/16 header bytes within the
previously validated stack, using aligned ptrace words. Marker version1 uses
three distinct fixed 16-byte messages (loaded, installed, native done); failure
cleanup drains at most48 bytes only for the new modes, versus16 for root-exec.
No bound or profile changes to make a test pass.

Record old normal entry/root baseline and new-mode red first. Test actual normal
root install/native and alternate-ABI outcomes; malformed event/header/policy,
duplicate/missing sequence, post-ND0 reader/phase faults and original SIGKILL/drop
at the installed boundary. Preserve synthetic-versus-real distinctions, seeded
mutation cases and unchanged policy-byte checks. Run targeted default-parallel
tests separately from the previously identified broad-load overlap, retaining
any failures; do not call isolated successes load robustness. Run current/MSRV
checks, strict lint, old entry/root/loss regressions and static contracts.

## Closed scope

No authority token, dummy/real credential handoff, production signing, independent
runtime custody, full normal-entry creator-loss proof, worker original-start,
P2/P3 all-writer revocation, completion/later cells, server or GitHub operation,
product measurement, release qualification or ship admission. Proc-less restriction
composition is not a complete sandbox or mapped-memory attestation.
