# Ordinary fixed GNU root preparation

The binary-private normal-build module prepares and removes a root containing
only signer, GNU loader, libc and libgcc. The fixed `prepare` CLI accepts no
caller-selected path, FD, PID, executable or command. It requires an empty
environment, a nonroot sole main thread and equal real/effective IDs. It never
forks, enters a root/namespace, signs, requests credentials or starts a worker.

Acquisition retains the original four read-only CLOEXEC source descriptors,
bounded bytes, SHA256 and metadata stamps (device/inode, mode, UID/GID, link
count, length, mtime/ctime including nanoseconds; atime excluded). The GNU
declaration parser and its 8 MiB/image bound are unchanged. Fixed system runtime
symlinks are deliberately followed, with numeric root ownership and non-writable
group/other modes checked. These checks do not authenticate initial namespace,
distribution ancestry, selected build provenance or independent runtime custody.
ACLs, xattrs and file capabilities are not attested by these stamps.

Creation uses a retained sticky root-owned `/tmp` and one 128-bit random name.
mkdirat/openat exclusive operations do not adopt collisions. Four retained
directory originals and four copied file originals are sealed to 0555; temporary
writable copy descriptors close before return. Rechecks require exact tree names,
original object identities, metadata, source/copy content and descriptor flags.
Refusal latches; a later successful observation cannot restore validity.
Source paths are not re-acquired or asserted as persistent name bindings.

Explicit cleanup preflights all known name bindings and bounded inventories before
chmod/unlink. It removes only known files/subdirectories and the fresh root, never
recursively follows a path. Foreign substitution/extra entries refuse before
mutation. Partial owned prefixes are covered. Failure acquiring a newly created
directory descriptor can leave an unowned name, not trigger deletion by guesswork.
An explicit cleanup failure prevents success and is not retried by Drop. Drop's
best effort is not crash cleanup. Process SIGKILL, durability and hostile same-UID
or privileged validation/unlink races are not solved by this cooperative local
preparation probe. File modes are not writer revocation or immutable custody.

The [evidence packet](local-runs/diagnostic-signer-owned-root-20261015/README.md)
separates synthetic graphs from actual normal-process preparation. Local Rust1.94
private debug copies, stripped without changing Cargo artifacts, prepare real
fixed GNU runtime images. Local Rust1.88 adds libm and deliberately refuses the
unchanged four-role graph; its unit materialization still passes. This is neither
MSRV runtime support nor release-profile/build attestation.

The earlier old-root broad baseline failed 78/1 in the constructor-loss seeded
observer. Focused 7/7 successes alone did not resolve it. The subsequent
[constructor peer barrier correction](diagnostic-signer-constructor-loss-design.md)
reproduces early EOF before creator death and keeps the before-arm gate held
until exact tracer termination/reap; repeated broader scoped gates now pass.
The original normal-root packet remains historical evidence of its own state.
The separate [ordinary root-exec probe](diagnostic-signer-owned-root-exec-design.md)
now joins selected originals with a normal original child, namespace/root transition
and initial/loaded kernel mappings, stopping before the restriction installer.
Existing child restrictions need procfs and must not be silently weakened inside
a proc-less root.
Full normal-entry loss, authenticated custody/handoff, original worker start and
P2/P3 writers remain open. Credentials, completion/cells 2–4 and server/ship
admission remain closed under the [contract](diagnostic-signer-owned-root-contract.toml).
