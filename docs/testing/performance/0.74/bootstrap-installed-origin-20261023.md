# Installed bootstrap origin stays under the original owner

The first approved production-boundary component separates a retained installed
bootstrap origin from the original sealed reference. A private wrapper consumes
the same BootstrapOwnedUnit and checks the fixed installed name, safe ancestry,
image metadata and content against that owner's verified build. No file is
installed and no original child, unit, transport or execution is enabled.

## Independent origin without weaker old checks

The [approved local plan](bootstrap-installed-origin-plan-20261023.md) follows
the [parent and child boundary audit](bootstrap-child-boundary-audit-20261023.md).
The future fixed name is
/opt/hydracache-perf/bin/diagnostic-held-bootstrap-074. Only the privileged parent
can open it; neither incoming documents nor environment nor caller paths select
an alternative. Production walks from / through fixed opt/hydracache-perf/bin
components with descriptor-relative readonly, CLOEXEC, nofollow, nonblocking
opens. Private filesystem tests use their own temporary modeled root and owner.
Ancestry requires both owner and other-search bits, so root-only 0700 traversal
is not accepted as a future non-root child pathname. This is necessary access
policy, not proof of successful exec or the child mount view.

The installed image must be a single-link regular file with root UID/GID,
mode 0555 or 0755, zero offset and the same bounded ELF/hash as the existing
verified bootstrap build. The source is not required to masquerade as the sealed
reference inode. It must instead be distinct from every old inventory object;
the old image's seals, role and child-image mode remain unchanged.

Both held and freshly named file metadata must match before and after content
inspection. Ancestry checks reject unsafe modes, ownership or inode replacement.
File/ancestor flags and zero file offset are retained, and hashing uses at most
128 MiB with a 64 KiB buffer and direct bounded pread. Read errors refuse without
unbounded EINTR retry. Root ownership is a source constraint inside the original
host context, not independent proof of host/namespace authority or immutable
content against a privileged adversary.

## Linear owner and expanded parent census

The new wrapper exposes no File/raw-FD/owner getter, runnable spec, key, start,
send, release or completion operation. It reuses the old complete validator:
old full frame, installed inspection, old full frame, installed inspection,
old closing full frame. Existing bootstrap trust, context, original state,
sealed reference and entry checks still run inside those frames with the same
original 300/60/10 origins and bounds. Extra work is not a new admission window.

The parent-local census preserves all 27 old FD values and fifteen semantic
roles, adding four retained ancestry descriptors and the distinct installed
image. The result is 32 physical FDs and sixteen roles, with exact prefix and
FD-collision checks and no image inode alias to any old object. Directory stamps
include conservative metadata, so unrelated directory changes may also refuse.
This is a retained value inventory, not an inherited child remapping table.

Any new error latches the source and retires the same original owner. Constructor
failure retains that refused wrapper/owner. Missing source after retention
refuses instead of recreating it; explicit retirement and owner-first field
ordering precede closing added descriptors. Models compile that ownership API
but cannot establish live cessation of every writer.

## Local controls and first outcomes

Five private filesystem tests exercise healthy repeat observations, permissions,
wrong owner, ELF/content/size drift, same-byte inode replacement, symlink,
hardlink, ancestor replacement, missing input, flags and offsets. Restoring an
original name does not heal a refused source. Census coverage retains the old
prefix and rejects duplicate FD and image aliases. Three owner models exercise
full-frame order, original origins, every frame/inspection cut, retained drift
and the consuming production signature without executing an original owner.

The [first results](bootstrap-installed-origin-first-result-20261023.json)
preserve the missing-owner prerequisite refusal and a development GNU warning
for an unused duplicate full-path constant. Removing that duplicate does not
alter the fixed component walker or suppress the warning. Fixed logs and source
snapshots remain distinct from captured-tool summaries.

The initial fixed GNU/MSRV controls passed but strict Clippy rejected a range
loop. The correction enumerates all four retained directories and gets the
three next path components without dropping a comparison or suppressing lint.
Source review also tightened the new ancestry policy from owner-only search
to owner/other search for the future unprivileged child. Fixtures now explicitly
set modeled roots to 0755 and reject 0700 roots. These are refinements to the
new nonexecuting component, not changes to old live predicates or performance
thresholds. First source snapshots and raw refusals remain in separate lanes.

All final lanes completed without failure on one stable source snapshot:

| Scope | Passed checks |
| --- | --- |
| Windows | 54 source contracts, 15 existing bootstrap/unit/process controls, all-target check, strict all-feature Clippy and affected-package formatting. |
| GNU 1.94 | Nine new filesystem/census/owner controls plus 64 preceding controls, all-target check, strict all-feature Clippy and exact selected unsigned build. |
| GNU MSRV 1.88 | The same 73 selected controls, all-target check and strict all-feature Clippy. |
| Metadata | Three Python controls, doc-check, 188 discovered/registered gates, local nonpromotable performance contract and diff checks. |

Implementation files are diagnostic_bootstrap_installed_source.rs,
diagnostic_bootstrap_installed_owner.rs and diagnostic_bootstrap_installed_census.rs
with their three test modules. Only three additive module hooks touch old
runtime files; exact views adapt historical source guards without removing any
old assertion or allowing unrelated drift. The additive fixed-origin contract
is registered in COMPAT, not installed or activated by it.

Source commit is 344bf09f660acd4dcae574ca32fe7ee9324a41d7. Evidence commit
9b71adf4 retains the exclusive
[inventory](local-runs/bootstrap-installed-origin-20261023/inventory.json),
SHA-256 471ab84626586a2be577d950678f1d3c19e7ac58ffd2502e87be3f64632c571d.
It covers 1150 members plus the inventory itself; all 1151 staged Git blobs
were verified against exact working bytes and inventory hashes before commit.
Initial and final source snapshots/logs are retained separately; earlier strict
lint failures are not rewritten into passes.

The selected unsigned GNU image was 1607888 bytes, SHA-256
0a03f6bbda964ed9e87e8575c38ad7702ab832a306a5c7b9e02e24360e0e0f07.
This checked dirty-worktree build is not a clean-source signed receipt or
installed bootstrap enrollment; a trusted selected clean build is still needed.
Additional pathname/hash/validation work is unmeasured: these correctness
controls do not show that original preparation fits its deadlines.
The qualification manifest remains unchanged at SHA-256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.

## Next production join

The approved amendment still needs a versioned child entry/inventory distinguishing
installed origin from sealed reference, binding current executable observation
to that origin without weakening any old mode. The actual held sandbox process
must then be admitted through the same authenticated original manager/owner,
including credentials, namespace objects, boot/generation/cgroup and streams
before release. No such authentication or wire v2 is implemented here.

All-writer cessation, durable completion, later cells and full SIGKILL/lost-ACK
coverage remain separate integration gates. No clock/service change, installed
production path, server run, product workload or performance cohort follows this
read-only component.
