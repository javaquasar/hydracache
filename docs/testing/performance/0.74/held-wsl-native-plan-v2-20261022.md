# Local WSL preparation revision 2 — environment versus harness

The first derivation refused `Required package absent: python3-minimal` before
any import. Its receipt, source and derived tar remain untouched. Read-only
inspection confirms installed systemd/systemd-sysv 257.13-1~deb13u1 and dbus
1.16.2-2; Python is not installed in the minimal image.

This revision changes only the environment preparation prerequisite: require
systemd, systemd-sysv and dbus; report optional Python separately. Write new
`debian-prepared-v2.tar` and `prepare-v2.json`, never replace the first attempt.
The first refusal is not a runtime fault or a failed service delivery; no service
has been submitted. Python-dependent future tests must refuse if it is absent.
No dependency installation is part of this revision.

All other [original plan](held-wsl-native-plan-20261022.md) constraints stand,
including pre-boot clock-agent masks, exclusive new name/path, unchanged global
configuration/default/Ubuntu and original 300/60/10 windows. This is not a
retry-to-pass of a native FD, original-owner or writer-cessation gate.
