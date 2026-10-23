"""Single read-only root-namespace lookup from an already elevated Windows shell."""
import argparse
import ctypes
import json
import os
import subprocess

import identity
import run as base

OUT = base.ROOT / 'target/wsl-clock-root-namespace-20261023'
IDENTITY_SHA = '4d2c26ffc6219eab7664de0e77d1b072392cf4e96434a6d8343cb1a0ce76fb7c'
WSL = r'C:\Windows\System32\wsl.exe'
SCRIPT = b'''if [ "$(id -u)" != 0 ]; then exit 40; fi
boot=$(cat /proc/sys/kernel/random/boot_id) || exit 41
printf 'OBSERVED_BOOT %s\\n' "$boot"
if [ "$boot" != cf222c07-5476-4f70-9a32-455e99204d0a ]; then exit 42; fi
pidns=$(readlink /proc/self/ns/pid) || exit 43
printf 'OBSERVED_PIDNS %s\\n' "$pidns"
case "$pidns" in 'pid:[4026532229]'|'pid:[4026532241]'|'pid:[4026532222]') exit 44;; esac
timens=$(readlink /proc/self/ns/time) || exit 45
printf 'OBSERVED_TIMENS %s\\n' "$timens"
if [ "$timens" != 'time:[4026531834]' ]; then exit 46; fi
printf 'HC074_ROOT_METADATA_BEGIN\\n'
printf 'BOOT %s\\nOBSERVER_PIDNS %s\\nOBSERVER_TIMENS %s\\n' "$boot" "$pidns" "$timens"
emit() {
 printf '\\nFIELD %s\\n' "$1"
 shift
 if "$@"; then rc=0; else rc=$?; fi
 printf '\\nRESULT %s\\n' "$rc"
}
for pid in 1 210 1157 1126 1112 920; do
 printf '\\nPROCESS %s\\n' "$pid"
 if [ ! -d "/proc/$pid" ]; then printf 'ABSENT\\n'; continue; fi
 emit stat-before cat "/proc/$pid/stat"
 emit status cat "/proc/$pid/status"
 emit cgroup cat "/proc/$pid/cgroup"
 for leaf in exe ns/pid ns/time ns/mnt; do emit "$leaf" readlink "/proc/$pid/$leaf"; done
 emit executable-digest sha256sum "/proc/$pid/exe"
 emit stat-after cat "/proc/$pid/stat"
done
after=$(cat /proc/sys/kernel/random/boot_id) || exit 47
if [ "$after" != "$boot" ]; then exit 48; fi
printf 'HC074_ROOT_METADATA_END\\n'
exit 0
'''


def administrator():
    return os.name == 'nt' and bool(ctypes.windll.shell32.IsUserAnAdmin())


def inventory(raw):
    text = raw.decode('utf-16le' if b'\x00' in raw else 'utf-8')
    names = [line.strip('\ufeff\r\n ') for line in text.splitlines() if line.strip()]
    if len(names) != 2 or set(names) != {'Ubuntu', 'docker-desktop'}:
        raise ValueError('Running distro inventory changed; do not start or stop anything')
    return sorted(names)


def shell():
    # A timeout/partial exchange is retained as indeterminate, never redispatched.
    with (OUT / 'debug-shell.stdout.log').open('xb') as out, (OUT / 'debug-shell.stderr.log').open('xb') as err:
        try:
            rc = subprocess.run([WSL, '--debug-shell'], input=SCRIPT, stdout=out, stderr=err,
                                cwd=base.ROOT, timeout=30, check=False).returncode
        except subprocess.TimeoutExpired:
            rc = 124
    base.put(OUT / 'debug-shell.json', {'argv': [WSL, '--debug-shell'],
             'exit': rc, 'timeout_seconds': 30, 'retry': False,
             'interpretation': 'Current metadata only; field results may fail or PIDs may be absent'})
    if rc:
        raise RuntimeError(f'First root lookup exit {rc}; preserve all files, no retry')
    raw = (OUT / 'debug-shell.stdout.log').read_bytes()
    # Exit 0 alone does not prove that the supplied script ran in the debug shell.
    lines = raw.replace(b'\r\n', b'\n').splitlines()
    begin, end = b'HC074_ROOT_METADATA_BEGIN', b'HC074_ROOT_METADATA_END'
    if lines.count(begin) != 1 or lines.count(end) != 1 or lines.index(begin) >= lines.index(end):
        raise ValueError('Debug exchange lacks exact completion markers; preserve first result')


def collect():
    if not administrator():
        raise RuntimeError('Requires an already elevated Windows PowerShell; no WSL command was run')
    if subprocess.check_output(['git', 'status', '--porcelain'], cwd=base.ROOT).strip():
        raise ValueError('Commit sources and preserve a clean worktree first')
    proof = base.sources()
    identity.verify()
    if base.digest((identity.ARCHIVE / 'index.json').read_bytes()) != IDENTITY_SHA:
        raise ValueError('Changed first writer-identity archive')
    OUT.mkdir(parents=True, exist_ok=False)
    base.OUT = OUT
    base.put(OUT / 'intent.json', {'schema': 'hc074-readonly-root-namespace-intent-v1',
             'git': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=base.ROOT, text=True).strip(),
             'sources': proof, 'source_capture_sha256': identity.CAPTURE_SHA,
             'writer_identity_sha256': IDENTITY_SHA, 'original_windows': [300, 60, 10],
             'retry': False, **dict.fromkeys(identity.FALSE_FLAGS, False)})
    with (OUT / 'stdin.sh').open('xb') as stream:
        stream.write(SCRIPT)
    base.command('running-before', [WSL, '--list', '--running', '--quiet'], 15)
    before = inventory((OUT / 'running-before.stdout.log').read_bytes())
    try:
        shell()
    finally:
        base.command('running-after', [WSL, '--list', '--running', '--quiet'], 15)
        after = inventory((OUT / 'running-after.stdout.log').read_bytes())
        if after != before or base.sources() != proof:
            raise ValueError('Postflight inventory or frozen sources changed; no retry')
    print(json.dumps({'output': str(OUT), 'exchange_completed': True,
                      'admission': False, 'historical_identity_authenticated': False}))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('collect',))
    parser.parse_args()
    collect()
