# Bootstrap child enrollment needs a sandbox and executable origin contract

The next credentials join cannot reuse the parent trust openers unchanged or
assume the child's mount namespace equals the host's. The existing sealed-image
consistency mode also does not supply an executable pathname for systemd.
Production integration needs an explicit contract amendment; this audit changes
no runtime behavior and does not open admission.

## Findings at the original trust boundary

Baseline cb7f639d retains distinct owners and origins:

| Existing boundary | Source fact | Integration consequence |
| --- | --- | --- |
| Privileged trust selection | open_fixed_worker_trust and open_fixed_held_runtime_trust require effective UID and GID zero before opening the fixed root source. | They are not unprivileged child openers. Removing that guard is a new authority surface, not a credentials fix. |
| Configured child identity | diagnostic_unit selects User/Group hydracache-perf and NoNewPrivileges. The held projection preserves those properties. | The configured worker is not the original root creator; HCBT0741 owner credentials describe the creator, not the child's required UID/GID. |
| Sandboxed mount view | PrivateTmp, PrivateDevices and ProtectSystem=strict remain in the projected unit. | The child's mount namespace must not be inferred to equal the parent's enrolled host namespace. |
| Existing namespace guard | pin_same_mount_namespace requires worker and observer equality, then matches the exact signed worker-policy namespace. | This is a same-namespace observer, not enrollment of a newly sandboxed systemd child. |
| Bootstrap image | The parent retains an exactly sealed bootstrap object. The image-only child mode requires current exe to equal that inode. The projection still has no runnable ExecStart. | A path to another file with identical bytes does not satisfy that diagnostic mode; FD13 has not become an executable address for systemd. |

The systemd manual documents that PrivateTmp, PrivateDevices and ProtectSystem
enable filesystem namespacing. That supports the design constraint, not a live
observation of a HydraCache unit or a particular installed systemd version.
[systemd.exec filesystem namespacing](https://www.freedesktop.org/software/systemd/man/latest/systemd.exec.html)

The earlier [current-image controls](bootstrap-current-image-20261023.md) remain
valid within their stated synthetic scope: the matching sealed diagnostic copy
refuses authority, and an on-disk executable beside that copy refuses image
consistency. They are not a production launch recipe or completed provenance
join. A new root executable or namespace has not been tested here.

## Proposed production amendment awaiting approval

Preserve all unit hardening and root-only openers. Extend only the separately
enrolled bootstrap/child protocol, rather than weakening an existing mode.

1. Give the separately trusted bootstrap a root-controlled installed executable
   origin, bound to the exact approved build/receipt and retained executable
   inode. Distinguish that origin from a sealed reference copy. A new inventory
   and protocol version must identify both roles explicitly; never silently
   reinterpret FD13 or make matching hashes replace required inode identity.
2. Have the same privileged original owner observe the specific held child
   through the authenticated original manager and pidfd/generation binding.
   Bind actual credentials, user and sandbox mount namespace objects, boot,
   cgroup, image and original streams before any worker release. The sandbox
   namespace is attempt-scoped, not a new host-policy namespace or caller claim.
3. Bind a bounded, one-shot child-enrollment exchange to that same owner/channel,
   attempt, policy/epoch and original deadline. This requires an explicit
   authentication and replay contract; peer root UID or a child-supplied namespace
   number alone is insufficient. Do not introduce a server-side signing-key
   default, new time origin or credential/namespace repair.

This is a recommendation for approval, not implemented authentication. The
alternative is a separately reviewed FD-native systemd executable-addressing
mechanism preserving the sealed image as the actual executable. Merely using
the creator's /proc pathname, retrying the rejected bind, running the child as
root or disabling sandbox properties is not an accepted substitute.

Approval would authorize local design/implementation and bounded fault controls,
not installation, service changes, server runs or product workload. Original
delivery, all-writer cessation, durable completion and the SIGKILL/lost-ACK
matrix still depend on the resulting integration gate.

## Audit guards and verification scope

bootstrap_child_boundary_074 adds four portable source-contract tests. They
check both root gates and mutation refusals, retained child hardening, the
same-namespace guard, the sealed-inode/address distinction and exact equality
of twelve original runtime/model files plus lock, COMPAT, registry and frozen
qualification against cb7f639d. These are source assertions, not positive child
credential or namespace authentication.

The fixed runner selects only those guards, affected xtask check/strict Clippy,
ten existing pure projection/role-view models and six bootstrap build/private
sealed-image tests on GNU/MSRV, then format/docs/registry/local nonpromotable contract checks. It
retains exact input source snapshots and logs without running an original live
fixture, bootstrap process control, namespace mutation or measured workload.
The first retainer expected only the seven unit-projection tests and refused
before commit lookup or packet creation when the unchanged selector selected
three earlier role-view tests too. Its
[captured failure](bootstrap-child-boundary-first-result-20261023.json) and exact
first source/logs remain retained. The correction counts all ten tests; it does
not change the selector, runtime or a performance acceptance threshold. Separate
final correctness lanes use the corrected evidence tooling.

All final lanes passed: four portable source guards plus all-target check and
strict all-feature Clippy for xtask on Windows; sixteen selected model/build/
private-image controls on each of GNU 1.94 and MSRV 1.88; three Python controls,
doc-check, 188 registered gates, local nonpromotable performance contract,
affected-package format and diff checks. These are not new systemd or child
credential/namespace fault controls.

Checked source is 29b24b90d619cb58f25286663f1b209e9ef2e68b. Evidence commit
eb87adf7 retains the first and final exact source/logs in the exclusive
[packet inventory](local-runs/bootstrap-child-boundary-20261023/inventory.json).
Its SHA-256 is 72cf38a0be3361a452a1af43df6ff6fece8857a101db7e611c50598108f77379,
covering 271 members plus the inventory itself. All 272 staged Git blobs were
verified against exact working bytes and inventory hashes before commit.
The first retainer failure remains a separate captured-tool summary rather than
a raw receipt. No performance, original-start or server-readiness claim follows.

The frozen qualification manifest remains SHA-256
11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc.
