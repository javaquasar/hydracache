"""Bounded WSL manager compatibility refusal, never an original-start permit."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[2]
MANAGER_PATH = '/org/freedesktop/systemd1'
MANAGER = 'org.freedesktop.systemd1'
BUS = 'org.freedesktop.DBus'
PREFIX = 'hc074-fdcompat-'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'
SCHEMA = 'held-manager-compatibility-074-v3'


def need(value, reason):
    if not value:
        raise ValueError(reason)


def publish(path, value):
    with path.open('x') as stream:
        json.dump(value, stream, sort_keys=True, indent=2)
        stream.write('\n')
        stream.flush()
        os.fsync(stream.fileno())


def scalar(receipt, kind):
    need(receipt['exit'] == 0, 'Manager query failed')
    value = json.loads(receipt['stdout'])
    need(set(value) == {'type', 'data'} and value['type'] == kind, 'Unexpected bus reply shape')
    need(isinstance(value['data'], list) and len(value['data']) == 1, 'Exactly one method reply argument required')
    need(type(value['data'][0]) is (str if kind in ('s', 'o') else int), 'Unexpected bus reply value')
    return value['data'][0]


def unique(name):
    return isinstance(name, str) and len(name) <= 65 and re.fullmatch(r':[0-9]+\.[0-9]+', name)


def version(value):
    need(isinstance(value, str) and re.fullmatch(r'[1-9][0-9]{2}(?:[. +~-][A-Za-z0-9.+~ -]{1,96})?', value),
         'Unrecognized manager version')
    return int(value[:3])


def absent(receipt, unit):
    return receipt['exit'] != 0 and receipt['stdout'] == '' and receipt['stderr'].strip() == f'Call failed: Unit {unit} not loaded.'


def refusal(receipt):
    # Do not classify access denial, malformed signatures, timeouts, or a generic
    # launch failure as evidence that this property is unsupported.
    return (receipt['exit'] != 0 and receipt['stdout'] == '' and
            receipt['stderr'].strip() == 'Call failed: Cannot set property ExtraFileDescriptors, or unknown property.')


def context():
    need(sys.platform == 'linux' and ROOT.as_posix() == '/mnt/c/Workspace/prj/jq/cashe/hydracache-074', 'Exact local WSL workspace required')
    need(os.getuid() == os.geteuid() == 0 and os.getgid() == os.getegid() == 0, 'Local root required for this isolated probe')
    need(os.uname().release.lower().endswith('microsoft-standard-wsl2'), 'Not local WSL')
    need(Path('/proc/1/comm').read_text() == 'systemd\n' and Path('/proc/1').stat().st_uid == 0, 'Original local system manager missing')
    qualification = hashlib.sha256((ROOT / 'docs/testing/performance/0.74/qualification-manifest.toml').read_bytes()).hexdigest()
    need(qualification == QUALIFICATION, 'Qualification changed')
    git = ['/mnt/c/Program Files/Git/cmd/git.exe', '--no-replace-objects', '--no-optional-locks',
           '-C', 'C:/Workspace/prj/jq/cashe/hydracache-074']
    head = subprocess.run([*git, 'rev-parse', 'HEAD'], capture_output=True, check=True, timeout=10).stdout.decode().strip()
    dirty = subprocess.run([*git, 'status', '--porcelain'], capture_output=True, check=True, timeout=10).stdout
    need(re.fullmatch(r'[0-9a-f]{40}', head) and not dirty, 'Committed clean compatibility method required')
    sources = ['tools/held-manager-compat-074/' + name for name in ('probe.py', 'test_probe.py', 'README.md')]
    sources.append('docs/testing/performance/0.74/held-manager-compat-plan-20261022.md')
    sources.append('docs/testing/performance/0.74/held-manager-compat-plan-v2-20261022.md')
    sources.append('docs/testing/performance/0.74/held-manager-compat-plan-v3-20261022.md')
    return dict(boot=Path('/proc/sys/kernel/random/boot_id').read_text(), kernel=os.uname().release,
                uid=os.getuid(), gid=os.getgid(), qualification=qualification, source=head,
                source_sha256={name: hashlib.sha256((ROOT / name).read_bytes()).hexdigest() for name in sources},
                busctl_sha256=hashlib.sha256(Path('/usr/bin/busctl').read_bytes()).hexdigest(),
                namespaces={name: os.readlink('/proc/self/ns/' + name) for name in ('mnt', 'pid', 'user', 'cgroup', 'time')})


class Bus:
    def __init__(self, output):
        self.output = output
        self.ordinal = 0
        self.deadline = time.monotonic() + 10  # Test guard only, no original admission is created or renewed.

    def call(self, destination, path, interface, member, signature, *args):
        remaining = self.deadline - time.monotonic()
        need(remaining > 0, 'Compatibility test budget exhausted')
        argv = ['/usr/bin/busctl', '--json=short', '--timeout=250ms', '--auto-start=no', 'call',
                destination, path, interface, member, signature, *args]
        index = self.ordinal
        self.ordinal += 1
        row = dict(argv=argv, exit=None, stdout='', stderr='', failure=None)
        try:
            child = subprocess.run(argv, env=dict(os.environ, LC_ALL='C'), capture_output=True, timeout=min(2, remaining))
            row.update(exit=child.returncode, stdout=child.stdout.decode(), stderr=child.stderr.decode())
        except Exception as error:
            row['failure'] = type(error).__name__ + ': ' + str(error)
            raise
        finally:
            publish(self.output / f'{index:03d}-command.json', row)
        need(time.monotonic() < self.deadline, 'Compatibility test budget exhausted after reply')
        return row


def owner(bus):
    name = scalar(bus.call(BUS, '/org/freedesktop/DBus', BUS, 'GetNameOwner', 's', MANAGER), 's')
    need(unique(name), 'Manager owner is not unique')
    pid = scalar(bus.call(BUS, '/org/freedesktop/DBus', BUS, 'GetConnectionUnixProcessID', 's', name), 'u')
    uid = scalar(bus.call(BUS, '/org/freedesktop/DBus', BUS, 'GetConnectionUnixUser', 's', name), 'u')
    need(pid == 1 and uid == 0, 'Manager is not the original PID 1 UID 0')
    return name


def manager_version(receipt):
    need(receipt['exit'] == 0, 'Manager version query failed')
    value = json.loads(receipt['stdout'])
    need(set(value) == {'type', 'data'} and value['type'] == 'v', 'Unexpected property reply')
    need(isinstance(value['data'], list) and len(value['data']) == 1, 'Exactly one version variant required')
    body = value['data'][0]
    need(isinstance(body, dict) and set(body) == {'type', 'data'} and body['type'] == 's', 'Unexpected version variant')
    version(body['data'])
    return body['data']


def check(bus, unit):
    need(re.fullmatch(PREFIX + r'[0-9a-f]{32}\.service', unit), 'Unsafe private unit')
    manager = owner(bus)
    value = manager_version(bus.call(manager, MANAGER_PATH, 'org.freedesktop.DBus.Properties', 'Get', 'ss', MANAGER + '.Manager', 'Version'))
    # This one negative experiment is preregistered for the observed 255 host.
    # A newer/unknown manager requires actual descriptor controls, never a green
    # result based solely on a version string or an empty descriptor array.
    need(version(value) == 255, 'This negative probe is only defined for systemd 255')
    before = bus.call(manager, MANAGER_PATH, MANAGER + '.Manager', 'GetUnit', 's', unit)
    need(absent(before, unit), 'Private unit already exists or absence was not proved')
    outcome = bus.call(manager, MANAGER_PATH, MANAGER + '.Manager', 'StartTransientUnit', 'ssa(sv)a(sa(sv))',
        unit, 'fail', '6',
        'Description', 's', unit[:-8],
        'Type', 's', 'oneshot',
        'Restart', 's', 'no',
        'NoNewPrivileges', 'b', 'true',
        'ExtraFileDescriptors', 'a(hs)', '0',
        'ExecStart', 'a(sasb)', '1', '/usr/bin/false', '1', '/usr/bin/false', 'false',
        '0')
    # Always inspect the exact name after possible delivery. Never resend, stop
    # a pre-existing unit, or infer no execution merely from a lost/error reply.
    after = bus.call(manager, MANAGER_PATH, MANAGER + '.Manager', 'GetUnit', 's', unit)
    need(absent(after, unit), 'Uncertain private unit state; retain receipt and inspect exact unit before cleanup')
    need(owner(bus) == manager, 'Manager changed during compatibility probe')
    need(refusal(outcome), 'First outcome was not the exact unsupported-property refusal')
    return dict(manager=manager, version=value, unit=unit, unsupported_property='ExtraFileDescriptors',
                disposition='blocked-unsupported-manager-fd-api', unit_absent_before=True, unit_absent_after=True,
                original_delivery=False, writers_cessation=False, admission_allowed=False, fallback=False,
                sent_descriptor_count=0, actual_descriptor_mapping_verified=False)


def run(attempt):
    need(re.fullmatch(r'[a-z0-9-]{1,40}', attempt), 'Unsafe attempt')
    before = context()
    output = ROOT / 'target' / ('held-manager-compat-' + attempt)
    need(not output.exists(), 'Never overwrite or resume a compatibility attempt')
    output.mkdir()
    result = dict(schema=SCHEMA, failure=None, before=before, server=False, product=False,
                  clocks_changed=False, existing_services_changed=False, original_delivery=False,
                  writers_cessation=False, admission_allowed=False)
    unit = PREFIX + secrets.token_hex(16) + '.service'
    # This local experiment intent is NOT the missing original durable send latch.
    publish(output / 'intent.json', dict(unit=unit, possible_helper='/usr/bin/false', rights=0, attempts=1))
    try:
        result['observation'] = check(Bus(output), unit)
        result['after'] = context()
        need(result['after'] == before, 'Local context changed')
    except Exception as error:
        result['failure'] = type(error).__name__ + ': ' + str(error)
    publish(output / 'result.json', result)
    need(result['failure'] is None, result['failure'])
    print(json.dumps(result, sort_keys=True))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--attempt', required=True)
    run(parser.parse_args().attempt)
