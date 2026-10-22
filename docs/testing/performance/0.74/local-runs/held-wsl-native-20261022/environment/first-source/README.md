# Isolated local native-FD environment preparation

This tool prepares a pinned Debian 13 rootfs for a **new local WSL distro**.
It does not import, boot, update, unregister or change any WSL environment.
Neither this environment nor a compatible systemd version supplies original
delivery authority, writer-cessation evidence, release admission or a server gate.

The exact image URL, SHA-256 and expected byte count are fixed in `prepare.py`.
The fetch must match the Microsoft distribution manifest. The original download
is retained unchanged. Offline derivation preserves other archive entries while
replacing only distro-local `wsl.conf`, removing vendor OOBE configuration and
masking clock agents and background maintenance timers **before first boot**.
Required installed packages and systemd >=257 are checked in the image.
Receipts are exclusive-create; neither mode resumes or overwrites a first result.

Offline controls (no WSL or network):

```powershell
python -B -m unittest discover -s tools/held-wsl-native-074 -p test_prepare.py -v
```

Preparation, once each in the repository root:

```powershell
python -B tools/held-wsl-native-074/prepare.py --fetch
python -B tools/held-wsl-native-074/prepare.py --derive
```

Outputs are ignored under `target/held-wsl-native-environment-20261022`.
The original and prepared rootfs are local environment artifacts, not Git data.
Keep their receipts and digests in repository evidence instead of committing tar
files or the virtual disk. Import/boot is a separately recorded operator step
under the [local environment plan](../../docs/testing/performance/0.74/held-wsl-native-plan-20261022.md).
