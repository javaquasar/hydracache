# Separate local WSL native-FD environment — 2026-10-22

## Scope and authority

The user authorized a separate local WSL environment with systemd 257+ after
the current Ubuntu manager refused `ExtraFileDescriptors`. This is environment
preparation, not a replacement/bootstrap protocol and not original delivery.
All runtime checks, predicate order/count, original 300/60/10 windows and first
refusals remain unchanged. No server, supervisor, product workload, performance
cohort or existing Ubuntu service/configuration change is authorized here.

Use only the new name `HydraCache074Native` and storage directory
`C:/Workspace/prj/jq/cashe/hydracache-074-wsl-native`. Never change the default
distro, global `.wslconfig`, installed Ubuntu or docker-desktop. Do not invoke
global WSL shutdown/update. Refuse name/path collisions. Keep at least 20 GiB
free before preparation. The new directory is environment storage, not a worktree.

## Pinned inputs and first-boot safety

The [Microsoft manifest](https://raw.githubusercontent.com/microsoft/WSL/master/distributions/DistributionInfo.json)
pins Debian WSL AMD64 1.26.0.0 at
`https://salsa.debian.org/debian/WSL/-/jobs/9606244/artifacts/raw/Debian_WSL_AMD64_v1.26.0.0.wsl`:
89,228,801 bytes, SHA-256
`5ec7dc68216e75d1d4d4761474e99d8461a98d316537110314b137122a879e0f`.
Check downloaded bytes and installed package metadata independently. Debian's
[trixie package](https://packages.debian.org/en/trixie/systemd) documents the
257 series; it is not evidence of the image's actual installed version.

Derive the image offline with the reviewed tool. Keep the original gzip archive
unchanged. Enable systemd only in the new distro's `/etc/wsl.conf`, remove vendor
OOBE and mask time agents/timedated and maintenance timers before boot. Never
manually change clocks or compensate failures by retrying with longer windows.
Record original/prepared digests and exact source. No extraction onto Windows.

## Local stages and stop conditions

1. Pass synthetic archive/refusal controls and source syntax checks. Preserve
   first fetch/derivation errors; review rather than silently retrying.
2. Record current WSL inventory/default/registry and global config digest or
   absence. Require new name/path absent. Import only the prepared archive using
   `wsl --import ... --version 2`, with a bounded command and retained receipt.
3. First boot: observe PID1/systemd/package versions, kernel/boot/clock, masks,
   read-only manager identity and native FD API availability. Compare existing
   registry/config state after import. A failure is not original-path evidence.
4. Before any temporary unit start, preregister a separate native FD mechanics
   control with fixed cases, scoped cleanup and immutable receipts. Actual
   retained connection, role mappings, process/streams and cgroup need checks;
   systemd version or a successful empty FD property alone is insufficient.
5. Original-owner delivery/writer cessation remains closed until the consumed
   owner/full validation frame, one-shot uncertainty, independent watchdog and
   all-writer closure are integrated and fault-tested. No ACK/cgroup-empty-only
   completion or fence release; no retry-to-pass or original send during setup.

Installation/import errors must retain receipts and stop dependent stages. A
temporary environment compatibility success does not modify qualification
manifest, release registry, frozen evidence or runtime code. Unregister/delete
of the new environment is not part of this preparation step.
