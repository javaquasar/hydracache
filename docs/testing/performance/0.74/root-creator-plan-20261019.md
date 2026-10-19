# Root guardian creator death and original frame join

The baseline is commit `4e940734512960999000245384857acadf9bb2f0`.
The independent local guardians have passed channel-loss controls, not real
creator death. No original root guardian capability exists at this baseline.
The qualification manifest and historical evidence packets remain immutable.

## Real creator death

Use a separate Python subreaper, an exec-created Rust creator and the existing
root guardian implementation. The harness owns a fresh local cgroup, fence and
absolute BOOTTIME timer. It receives both exact guardian pidfds through a
credential-checked test-only carrier before allowing enrollment. Cut the actual
creator with its own pidfd after the first ACK, second ACK, local armed assignment,
API return, and both queued but unread ACKs. Check actual SIGKILL status, early
owner-loss revocation, late population, retained original flock, single guardian
loss/stall and common-mode negatives. An external pipe alias must still block EOF.
All controlled descendants must be terminated and reaped before removing only
the fresh fixture directories; restore the previous subreaper setting.

## Original frame join

Derive fence, existing timer, deadline and context from the opaque original owner,
not caller claims. Bracket preparation and enrollment with the unchanged complete
validation round and retain the original exclusive borrow on uncertainty. Keep
the v1 direct-child guardian and existing checks byte-for-byte through exact
additive adapters. Original root admission, held-unit execution, completion and
fence release remain closed until their own gates are proved.

## Acceptance

Keep a failing baseline and same-source GNU current/MSRV local repetitions,
strict lint, source-preservation checks, registry and performance-contract checks.
Record preliminary failures as failures, not successful cohorts. No server
contact, installation, supervisor operation, original send or product workload
is part of this package. Preserve all original 300/60/10 origins and effect
brackets; runtime code must not create, reset or consume the original timer.

## Implemented boundary

The external harness is implemented in `52d805da`; the original frame adapter
is implemented in `5468524b`. The frame test deliberately retries enrollment
after its healthy pass, so `c1799824` corrects its outer expectation to require
original refusal in that case too. This changes no runtime predicate or deadline.
`f48fd4bf` additionally checks queued Armed bytes through test-only MSG_PEEK,
without consuming their acknowledgements.

The root adapter retains an actual pinned root and the opaque original owner
supplies fence, timer and a digest derived from its retained source. Preparation
uses two complete rounds; enrollment uses two more, with kernel observations
around the same rounds and sticky refusal on either side. Revalidation uses one
complete round. No original start/Armed execution capability or release API is
provided. A pinned kernel root is not production host admission.

Two dynamic gates remain deliberately distinct: nonroot original full-frame
controls use a simulated kernel, while root-only controls exercise the real
adapter and actual cgroup kills. Combining them with a privileged original
manager/host-root fixture, and admitting the exact root from protected host
context, remains required before a production original constructor is usable.
Existing nonroot fixture restrictions are not removed to obtain a green test.
