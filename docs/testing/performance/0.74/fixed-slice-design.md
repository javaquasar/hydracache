# Fixed slice path identity and original custody

The [local plan](fixed-slice-plan-20261020.md) adds a read-only path check to the
[same-owner placement join](owned-placement-design.md). A signed expected name
does not prove where the retained revocation root actually lives. In particular,
the temporary fixture root and a direct worker in system.slice are not the
attempt-specific slice described by the v3 requirements.

## Retained and named directories

The observer derives system-hc074 plus the full lowercase attempt digest and
.slice, using the existing enrollment identity. It opens each component from
slash through sys, fs and cgroup without following symlinks. Directory objects
must be owned by root with neither group nor other write permission. The final
mount, system.slice and attempt child must be cgroup2 on the same device.

Both the named child and the originally retained root must have the same full
device, inode, ownership and mode stamp. The original retained parent must be
the observed system.slice. Fresh named traversal and the retained directory
descriptors must agree on revalidation. The observer never replaces the old
root, events or kill descriptor, and never creates a directory or writes a
cgroup control. A namespace or path replacement cannot be accepted merely
because its basename resembles the signed name.

These are observations within the original namespace context, not proof that
the systemd manager owns this slice or that population remains exclusive between
observations. They do not protect against a privileged peer changing the tree
between observations. Original namespace/manager checks remain necessary, as
do controlled setup, writer admission and revocation before delivery.

## One original owner and unchanged live frame

The private constructor consumes the whole OwnedPlacement. It accepts no path,
key, inode, timer or replacement owner from its caller. The original armed
guardian pair supplies the root, and the retained signed snapshot supplies the
attempt. Opening and closing path observations surround the existing placement
join with its one unchanged complete live round. Signature verification, exact
consent/context/bound comparison and every old root observation still execute.

On success the new holder retains the entire consumed chain and read-only path
anchors. On failure it poisons both original owners and retains the whole old
holder in the error; it does not return a fresh attempt or descriptor. Destruction
keeps the old alias/custody retirement order. There is no send, release, completion
or unlock method, no new admission clock and no change to the original bounds.

## Scope of the local checks

Models cover ordering and refusal of the new join, including a failed full frame
and original-context drift during that frame. Kernel controls reject symlinks,
unsafe components, writable directories and regular filesystem substitutes.
The explicitly manual check reads existing local cgroup2/system.slice anchors
and verifies read-only, close-on-exec descriptors. It creates no attempt slice.

A successful model or anchor read is not an accepted actual fixed-root join.
That positive kernel path, systemd ownership/exclusive population, authenticated
launcher/watchdog/collector objects and the complete held-unit projection remain
separate gates. Original delivery, all-writer cessation, durable lifecycle,
same-attempt reconnect and server enrollment are not opened by this package.

## Verification result and remaining integration failure

Selected source c59b677f passes all 321 source contracts, preserving all 432 old
supervisor sources and old wire/harness sections through exact additive hooks.
GNU 1.94 and MSRV 1.88 each pass nine new and 68 retained ordinary controls,
247 doc tests, strict lint and the separate read-only local anchor check.
The [packet](local-runs/fixed-slice-20261020/README.md) retains source/image/recipe
identity and initial import, adapter and needless-borrow refusals independently.
No actual positive fixed-root join or new original-frame live matrix is claimed.

The broader Windows integration invocation exposes an existing synthetic
frozen-lock fixture mismatch: reconstructing current Cargo.lock produces
629b483a…7b99 instead of the required be46eaf8…2b19. The unchanged baseline
bytes reproduce that mismatch; the original pinned worker source still provides
the correct lock hash. Scoped portable library/doc tests and lint pass, but the
broader integration suite remains red. Fix the synthetic fixture separately with
the exact frozen bytes, never by changing the production pin or lowering a gate.

That separate [fixture repair](frozen-lock-fixture-design.md) subsequently fixes
both synthetic consumers and restores the complete ordinary Windows monitor
package on 84fee2cc. The sealed c59b677f refusal remains historical evidence;
neither the later ordinary green result nor this read-only observer closes
actual fixed-root, manager ownership, artifact or held-unit admission.
