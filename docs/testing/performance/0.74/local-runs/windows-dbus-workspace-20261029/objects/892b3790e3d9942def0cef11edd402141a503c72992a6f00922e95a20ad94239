"""One-shot import/first boot of the exact new local distro, never existing WSL."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time
import winreg
import prepare as p

PREPARED_SHA = '253025e7599908f372199e8515ba0a1574b7dc25b0b1104f16ba5587a926a2d9'
REGISTRY = r'Software\Microsoft\Windows\CurrentVersion\Lxss'


def snapshot():
    result = dict(default=None, distros={})
    with winreg.OpenKey(winreg.HKEY_CURRENT_USER, REGISTRY) as key:
        result['default'] = str(winreg.QueryValueEx(key, 'DefaultDistribution')[0])
        for index in range(winreg.QueryInfoKey(key)[0]):
            name = winreg.EnumKey(key, index)
            with winreg.OpenKey(key, name) as child:
                row = {}
                for entry in range(winreg.QueryInfoKey(child)[1]):
                    field, value, kind = winreg.EnumValue(child, entry)
                    row[field] = dict(kind=kind, value=value.hex() if isinstance(value, bytes) else value)
                result['distros'][name] = row
    config = Path(os.environ['USERPROFILE']) / '.wslconfig'
    result['global_config_sha256'] = p.sha(config) if config.exists() else None
    result['free_bytes'] = p.shutil.disk_usage(p.ROOT).free
    return result


def preserved(before, after):
    p.need(before['default'] == after['default'], 'Default distro changed')
    p.need(before['global_config_sha256'] == after['global_config_sha256'], 'Global WSL config changed')
    for key, value in before['distros'].items():
        p.need(after['distros'].get(key) == value, 'Existing distro registry changed')
    added = [row for key, row in after['distros'].items() if key not in before['distros']]
    p.need(len(added) == 1 and added[0].get('DistributionName', {}).get('value') == p.DISTRO,
           'Not exactly the one intended new distro')
    base = Path(added[0]['BasePath']['value'].removeprefix('\\\\?\\'))
    p.need(base.resolve() == p.INSTALL.resolve(), 'New distro has wrong storage path')


def command(argv, timeout, name):
    start = time.monotonic_ns()
    row = dict(argv=argv, timeout_seconds=timeout, failure=None)
    try:
        child = subprocess.run(argv, capture_output=True, timeout=timeout)
        row['exit'] = child.returncode
        for stream, data in (('stdout', child.stdout), ('stderr', child.stderr)):
            with (p.OUTPUT / (name + '.' + stream + '.log')).open('xb') as output: output.write(data)
            row[stream + '_sha256'] = hashlib.sha256(data).hexdigest()
    except subprocess.TimeoutExpired as error:
        row.update(exit=None, failure='TimeoutExpired; no resubmit')
        for stream, data in (('stdout', error.stdout or b''), ('stderr', error.stderr or b'')):
            with (p.OUTPUT / (name + '.' + stream + '.log')).open('xb') as output: output.write(data)
    row['elapsed_ns'] = time.monotonic_ns() - start
    p.publish(p.OUTPUT / (name + '.json'), row)
    p.need(row['exit'] == 0, name + ' failed; inspect retained receipt, do not resubmit')


def run(mode):
    marker = p.OUTPUT / (mode + '-started.json')
    p.need(not marker.exists(), 'Never resume or retry an environment stage')
    p.need(p.sha(p.OUTPUT / 'debian-prepared-v2.tar') == PREPARED_SHA, 'Prepared image changed')
    before = snapshot()
    if mode == 'import':
        p.need(not p.INSTALL.exists(), 'New distro path must not exist')
        p.need(all(row.get('DistributionName', {}).get('value') != p.DISTRO for row in before['distros'].values()),
               'Distro name collision')
        p.need(before['free_bytes'] > 20 * 1024**3, 'At least twenty GiB free required')
    else:
        p.need(json.loads((p.OUTPUT / 'import-command.json').read_text())['exit'] == 0, 'Import did not succeed')
        preserved(json.loads((p.OUTPUT / 'import-before.json').read_text()), before)
    p.publish(p.OUTPUT / (mode + '-before.json'), before)
    p.publish(marker, dict(mode=mode, source_sha256=p.sha(Path(__file__)), no_retry=True))
    if mode == 'import':
        command(['wsl.exe', '--import', p.DISTRO, str(p.INSTALL), str(p.OUTPUT / 'debian-prepared-v2.tar'),
                 '--version', '2'], 180, 'import-command')
    else:
        script = '/mnt/c/Workspace/prj/jq/cashe/hydracache-074/tools/held-wsl-native-074/first_boot.sh'
        command(['wsl.exe', '-d', p.DISTRO, '-u', 'root', '--cd', '/', '--exec', '/bin/bash', script], 90, 'boot-command')
    after = snapshot()
    p.publish(p.OUTPUT / (mode + '-after.json'), after)
    original = before if mode == 'import' else json.loads((p.OUTPUT / 'import-before.json').read_text())
    preserved(original, after)
    p.publish(p.OUTPUT / (mode + '-result.json'), dict(success=True, existing_registry_preserved=True,
        global_config_preserved=True, default_preserved=True, original_delivery=False,
        writers_cessation=False, admission_allowed=False, server_operations=0))
    print(json.dumps(dict(stage=mode, success=True, distro=p.DISTRO), sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('mode', choices=('import', 'boot'))
    run(parser.parse_args().mode)
