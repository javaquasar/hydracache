# Rejected sealed launcher bind and independent bootstrap selectors

The approved bind hypothesis failed its first sealed-source precondition on the
local WSL kernel. Its unfinished runtime candidate was removed, not admitted by
substituting a regular file or copying the image. The user then approved a
separately trusted bootstrap. The first implementation step adds protected
bootstrap selectors; it does not execute that bootstrap or admit a server.

## First bind outcome remains red

The [initial plan](held-bind-alias-plan-20261023.md) required a read-only bind of
the same original sealed inode, retained through retirement. The sole private
namespace precondition used root with a uniquely owned temporary directory and
an actual sealed memfd. open_tree with AT_EMPTY_PATH/CLONE returned EINVAL on
6.6.87.2-microsoft-standard-WSL2, before a detached mount or publication succeeded.
There is no retry, alternative syscall, pathname fallback, copied inode or weakened
seal. The original feature integration gate remains red; its first source/output
and unfinished candidate are retained outside compilation as rejected text.

The [matching WSL kernel source](https://github.com/microsoft/WSL2-Linux-Kernel/blob/linux-msft-wsl-6.6.87.2/fs/namespace.c#L2440)
routes detached cloning through __do_loopback, which rejects source mounts outside
the caller namespace except namespace dentries. The internal shmem mount behind
memfd is consistent with this refusal. This is a source-class attribution, not
a claim that bind mounts are unavailable on all Linux versions.

One [separate ordinary-file control](held-bind-alias-attribution-plan-20261023.md)
showed the mount API working for a namespace-backed file. Its child published a
readonly/nosuid/nodev alias, closed source/tree FDs and unlinked the source path;
the external namespace keeper SIGKILLed and reaped that exact creator. Same inode,
bytes and readonly flags remained. This file was unsealed and explicitly ineligible
as the original launcher. The **whole control failed** afterward: ismount missed
a same-filesystem bind mount, so TemporaryDirectory cleanup returned EBUSY.
Its partial observation is not a passed original-source or retirement gate.

The namespace ended with the control; a subsequent read-only prefix audit found
no matching temporary fixture directories. An offline cleanup helper now tests
an unconditional exact-owned-leaf unmount attempt instead of ismount detection.
It is not used to rerun the control, nor is it a production cleanup API. No
privileged mount, creator or original-owner matrix is repeated after these two
first outcomes. A Windows offline test-import failure was separately corrected by
mocking CDLL at import; this captured-tool summary is not a raw Linux receipt.

The earlier bind recommendation was therefore not valid for the current source
object/kernel. No failed gate is relabeled successful and no production mount
cleanup, all-writer cessation or release-fence proof is claimed.

## First separately approved bootstrap step

The [bootstrap plan](held-bootstrap-enrollment-plan-20261023.md) introduces the
fixed protected file /etc/hydracache-perf/held-bootstrap-enrollment-074.json.
It selects a nonweak bootstrap builder key, bootstrap policy and binary digests,
bound to the same protected held policy and epoch. The new key must differ from
controller, worker builder/issuer and held builder. There is no default key/pin,
caller path, configuration writer or generated key. No such file is installed.

BootstrapEnrollment borrows the same HeldTrustInput and uses its retained safe
root/ancestor/file primitives. Fresh old held-source validations bracket new
snapshot comparison. Missing/malformed/oversize/unknown/noncanonical documents,
key reuse, policy/epoch mismatch, zero pins, unsafe mode/link count and source /
ancestor replacement cause shared sticky refusal. Restoring a file cannot heal
either guard. One exact module hook is the only change to the old trust source;
the complete old validation bodies and all original launcher/manager/material
checks and absolute windows remain unchanged.

This is an independently protected selector, not completed host enrollment,
build verification or original-owner admission. It exports neither its selected
key/FD/source nor start/send/release/completion/unlock rights. Root ownership
does not prove initial-host/kernel/manager identity. A purpose-bound bootstrap
policy/receipt and actual original consuming join remain mandatory before a
bootstrap can use one execveat on the exact sealed launcher FD. The bootstrap
executable, child admission protocol and that exec path are not implemented here.

## Focused verification

Five new local Linux tests cover canonical shape/key/policy/epoch refusals,
healthy retained input, file bytes/mode/inode/symlink drift, hardlinks/missing
input and original ancestor drift. Three old held-source/key/replacement tests
remain selected without an original-owner fixture. Current-source contracts
reconstruct the old trust source and preceding compatibility bytes, reject hook /
schema mutations, preserve the old activation gate and prove the rejected alias
has no runtime hook. Seven offline Python controls cover errno propagation,
same-filesystem cleanup logic, scoped check selection and evidence refusal.

No performance cohort or timing/allocation gain is measured. Compile/test elapsed
time is operational metadata. The fixed runner cannot execute either privileged
diagnostic again. The qualification digest remains
`11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc`.
No clock, service, supervisor, installation, server or product workload is changed.

The fixed check lanes completed successfully:

- Windows: 22 selected source-contract tests, check, all-feature strict Clippy
  and formatting.
- Local Linux GNU 1.94 and MSRV 1.88, each: five new selector tests and three
  old protected-source/key/replacement regressions, all-target check and
  all-feature strict Clippy. Default-feature check retains the preceding
  unused Reading::Reference test-support warning; it was not suppressed.
- Offline controls: seven Python tests; doc-check; gated-test-check with 186
  registered gates; local, non-promotable performance-contract-check; diff check.

These are selector/filesystem/source gates, not a new original-owner, manager,
bootstrap execution or all-writer fault matrix. The first sealed bind refusal
and whole ordinary-file cleanup failure remain failures.

The [exclusive evidence packet](local-runs/held-bind-bootstrap-20261023/audit.json)
checks source commit `c244473c9f878fe89920ec4c76c2bcd3b071a8e7`. Its
[inventory](local-runs/held-bind-bootstrap-20261023/inventory.json) contains 125
members (excluding the inventory itself), SHA-256
`65a96cfd51bd36c0a3e72df462bbbcb636a17f62b4ffd3623ea717a65ebd9643`.
It includes raw fixed-lane logs, diagnostic first failures, checked source bytes,
rejected text and the scoped runner/retainer. Plans/rejected design are in
`a3aa9773`; diagnostic/evidence tooling in `9c843520`; the protected selector,
its tests and additive compatibility registration in `c244473c`.
Evidence is committed separately in `9afe4af1`. A raw staged-Git-blob audit
verified all 126 files, including inventory, against their exact retained bytes
before that commit; newline conversion did not alter the packet.

Next implement the separately pinned bootstrap build policy/receipt and its
original-owner/frame join, then fixed child credentials/FD validation and one
execveat without fallback. Full original delivery, all-writer cessation,
SIGKILL/lost-ACK, completion/cells 2–4, reconnect and server admission remain open.
