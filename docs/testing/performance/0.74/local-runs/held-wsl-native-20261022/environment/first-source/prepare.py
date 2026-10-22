"""Prepare a pinned, clock-agent-masked rootfs without launching or importing WSL."""
import argparse
import copy
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import tarfile
import time
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
OUTPUT = ROOT / 'target/held-wsl-native-environment-20261022'
MANIFEST = 'https://raw.githubusercontent.com/microsoft/WSL/master/distributions/DistributionInfo.json'
IMAGE = 'https://salsa.debian.org/debian/WSL/-/jobs/9606244/artifacts/raw/Debian_WSL_AMD64_v1.26.0.0.wsl'
IMAGE_SHA = '5ec7dc68216e75d1d4d4761474e99d8461a98d316537110314b137122a879e0f'
IMAGE_BYTES = 89228801
DISTRO = 'HydraCache074Native'
INSTALL = Path('C:/Workspace/prj/jq/cashe/hydracache-074-wsl-native')
MASKS = tuple('etc/systemd/system/' + name for name in (
    'systemd-timesyncd.service', 'systemd-timedated.service', 'systemd-time-wait-sync.service',
    'chrony.service', 'chronyd.service', 'ntp.service', 'ntpd.service', 'ntpsec.service', 'openntpd.service',
    'apt-daily.timer', 'apt-daily-upgrade.timer', 'fstrim.timer', 'man-db.timer', 'e2scrub_all.timer', 'logrotate.timer'))
WSL_CONF = b'[boot]\nsystemd=true\n[network]\nhostname=hc074-native\n[interop]\nappendWindowsPath=false\n[user]\ndefault=root\n'
REPLACED = {'etc/wsl.conf', 'etc/wsl-distribution.conf', *MASKS}


def need(value, reason):
    if not value: raise ValueError(reason)


def sha(path):
    value = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''): value.update(chunk)
    return value.hexdigest()


def publish(path, value):
    with path.open('x') as stream: json.dump(value, stream, sort_keys=True, indent=2)


def member(name):
    while name.startswith('./'): name = name[2:]
    if name in ('', '.'): return ''
    need(not name.startswith('/') and '\\' not in name and '..' not in name.split('/') and
         str(PurePosixPath(name)) == name.rstrip('/'), 'Unsafe image member')
    return name.rstrip('/')


def derive(source, destination):
    need(not destination.exists(), 'Never overwrite derived image')
    seen = set(); copied = 0; total = 0
    with tarfile.open(source, 'r:gz') as archive, tarfile.open(destination, 'w', format=tarfile.PAX_FORMAT) as output:
        for original in archive:
            name = member(original.name)
            if not name: continue
            need(name not in seen, 'Duplicate image member')
            seen.add(name)
            if name in REPLACED: continue
            total += original.size
            need(total < 2 * 1024**3, 'Rootfs exceeds fixed two-GiB derivation bound')
            item = copy.copy(original)
            item.name = name
            output.addfile(item, archive.extractfile(original) if original.isfile() else None)
            copied += 1
        for name in ('etc/wsl.conf', *MASKS):
            item = tarfile.TarInfo(name)
            item.uid = item.gid = item.mtime = 0
            if name == 'etc/wsl.conf':
                item.mode = 0o644; item.size = len(WSL_CONF)
                output.addfile(item, io.BytesIO(WSL_CONF))
            else:
                item.type = tarfile.SYMTYPE; item.mode = 0o777; item.linkname = '/dev/null'
                output.addfile(item)
    with tarfile.open(destination, 'r:') as archive:
        for name in MASKS:
            item = archive.getmember(name)
            need(item.issym() and item.linkname == '/dev/null', 'Clock/background mask missing')
        need(archive.extractfile('etc/wsl.conf').read() == WSL_CONF, 'Wrong private WSL config')
        need('etc/wsl-distribution.conf' not in archive.getnames(), 'Vendor OOBE remains enabled')
        version = archive.extractfile('etc/debian_version').read().decode().strip()
        need(version.startswith('13.'), 'Pinned image is not Debian 13')
        status = archive.extractfile('var/lib/dpkg/status').read().decode()
        packages = {}
        for paragraph in status.split('\n\n'):
            fields = dict(line.split(': ', 1) for line in paragraph.splitlines() if ': ' in line and not line.startswith(' '))
            if fields.get('Package') in ('systemd', 'systemd-sysv', 'dbus', 'python3-minimal'):
                packages[fields['Package']] = dict(version=fields.get('Version'), status=fields.get('Status'))
        for package in ('systemd', 'systemd-sysv', 'dbus', 'python3-minimal'):
            need(packages.get(package, {}).get('status') == 'install ok installed', 'Required package absent: ' + package)
        major = re.match(r'^(?:[0-9]+:)?([0-9]+)[.\-]', packages['systemd']['version'] or '')
        need(major is not None and int(major[1]) >= 257, 'Native FD environment requires systemd 257 or later')
    return dict(copied_members=copied, original_payload_bytes=total, debian_version=version, packages=packages,
                masks=list(MASKS), excluded_vendor_oobe=True, wsl_conf_sha256=hashlib.sha256(WSL_CONF).hexdigest())


def fetch():
    need(not OUTPUT.exists(), 'Never overwrite or resume image fetch')
    need(shutil.disk_usage(ROOT).free > 20 * 1024**3, 'At least twenty GiB free required')
    OUTPUT.mkdir()
    result = dict(source_url=IMAGE, expected_sha256=IMAGE_SHA, expected_bytes=IMAGE_BYTES, failure=None,
                  distro=DISTRO, install=str(INSTALL), imported=False, existing_services_changed=False)
    try:
        raw = urllib.request.urlopen(MANIFEST, timeout=30).read(1024 * 1024)
        with (OUTPUT / 'distribution-info.json').open('xb') as stream: stream.write(raw)
        row = json.loads(raw)['ModernDistributions']['Debian'][0]['Amd64Url']
        need(row == dict(Url=IMAGE, Sha256=IMAGE_SHA), 'Microsoft image pin differs; review rather than follow latest')
        deadline = time.monotonic() + 300
        count = 0
        with urllib.request.urlopen(IMAGE, timeout=30) as response, (OUTPUT / 'debian-original.wsl').open('xb') as stream:
            need(int(response.headers['Content-Length']) == IMAGE_BYTES, 'Unexpected download size')
            for chunk in iter(lambda: response.read(1024 * 1024), b''):
                count += len(chunk)
                need(count <= IMAGE_BYTES and time.monotonic() < deadline, 'Download size/time bound exceeded')
                stream.write(chunk)
        need(count == IMAGE_BYTES and sha(OUTPUT / 'debian-original.wsl') == IMAGE_SHA, 'Rootfs digest refused')
        result['downloaded_sha256'] = IMAGE_SHA
    except Exception as error: result['failure'] = type(error).__name__ + ': ' + str(error)
    publish(OUTPUT / 'fetch.json', result)
    need(result['failure'] is None, result['failure'])
    print(json.dumps(result, sort_keys=True))


def build():
    need(sha(OUTPUT / 'debian-original.wsl') == IMAGE_SHA, 'Original image changed')
    result = dict(failure=None, original_sha256=IMAGE_SHA, imported=False)
    try:
        result.update(derive(OUTPUT / 'debian-original.wsl', OUTPUT / 'debian-prepared.tar'))
        result['prepared_sha256'] = sha(OUTPUT / 'debian-prepared.tar')
        result['prepared_bytes'] = (OUTPUT / 'debian-prepared.tar').stat().st_size
    except Exception as error: result['failure'] = type(error).__name__ + ': ' + str(error)
    publish(OUTPUT / 'prepare.json', result)
    need(result['failure'] is None, result['failure'])
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument('--fetch', action='store_true'); modes.add_argument('--derive', action='store_true')
    args = parser.parse_args()
    fetch() if args.fetch else build()
