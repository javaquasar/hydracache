"""Offline audit/retention of immutable receipts; no WSL or bus execution."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import prepare as p

PACKET = p.ROOT / 'docs/testing/performance/0.74/local-runs/held-wsl-native-20261022'
QUALIFICATION = '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc'


def load(path): return json.loads(path.read_bytes())


def call_rows(root):
    rows = []
    for file in sorted((root / 'calls').glob('*.argv')):
        rows.append(dict(argv=file.read_text(), exit=int(file.with_suffix('.exit').read_text()),
            stdout=file.with_suffix('.stdout').read_text(), stderr=file.with_suffix('.stderr').read_text()))
    p.need(len(rows) >= 5, 'Missing command receipts')
    p.need(rows[0]['stdout'] == 's ":1.0"\n' and rows[1]['stdout'] == 'u 1\n' and rows[2]['stdout'] == 'u 0\n',
           'Manager receipt identity differs')
    return rows


def audit():
    p.need(p.sha(p.ROOT / 'docs/testing/performance/0.74/qualification-manifest.toml') == QUALIFICATION, 'Qualification changed')
    protected = ['tools/long-run-supervisor-074/src', 'docs/plans/releases.toml',
                 'docs/testing/performance/0.74/qualification-manifest.toml']
    p.need(subprocess.check_output(['git', 'diff', 'f0c0d870', '--', *protected], cwd=p.ROOT) == b'', 'Protected inputs changed')
    p.need(load(p.OUTPUT / 'prepare.json')['failure'] == 'ValueError: Required package absent: python3-minimal', 'First refusal lost')
    prepared = load(p.OUTPUT / 'prepare-v2.json')
    p.need(prepared['failure'] is None and prepared['prepared_sha256'] == p.sha(p.OUTPUT / 'debian-prepared-v2.tar'), 'Derived image changed')
    for stage in ('import', 'boot'):
        result = load(p.OUTPUT / (stage + '-result.json'))
        p.need(result['success'] and not result['original_delivery'] and not result['writers_cessation'], 'Environment state wrong')
    first = p.ROOT / 'target/held-native-fd-20261022'; second = p.ROOT / 'target/held-native-fd-v2-20261022'
    a, b = call_rows(first), call_rows(second)
    starts_a = [row for row in a if ' StartTransientUnit ' in row['argv']]
    starts_b = [row for row in b if ' StartTransientUnit ' in row['argv']]
    p.need(len(starts_a) == len(starts_b) == 1, 'Not exactly one CLI invocation per source')
    p.need(starts_a[0]['exit'] == 1 and starts_a[0]['stderr'] == "busctl: unrecognized option '--helper'\n", 'First CLI refusal changed')
    p.need(starts_b[0]['exit'] == 0 and starts_b[0]['stdout'].startswith('o "/org/freedesktop/systemd1/job/'), 'No actual native ACK')
    p.need(sum(' StopUnit ' in row['argv'] for row in a) == 0 and sum(' StopUnit ' in row['argv'] for row in b) == 1, 'Unexpected stop route')
    p.need(b[-1]['stdout'] == 's "inactive"\n', 'Temporary service not observed inactive')
    errors = [row['stderr'] for row in b if ' ControlGroup ' in row['argv']]
    p.need(len(errors) == 1 and 'Unknown interface' in errors[0], 'First property refusal lost')
    helper = (second / 'helper.stdout').read_text()
    p.need('native-fd-mapping-ok\n' in helper and ':2:role:empty\n' in helper and 'executable=/usr/bin/bash\n' in helper,
           'Helper mechanics evidence absent')
    nonce = load(second / 'intent.json')['nonce']
    cgroup = '/system.slice/system-hc074nativefd' + nonce + '.slice/hc074-nativefd-' + nonce + '.service'
    p.need('0::' + cgroup + '\n' in helper, 'Wrong helper cgroup')
    for line in (second / 'stream-identities.txt').read_text().splitlines(): p.need(line + '\n' in helper, 'Stream identity differs')
    p.need((second / 'helper.stderr').read_bytes() == b'', 'Helper stderr not empty')
    for folder in (first, second):
        row = load(folder / 'result.json')
        p.need(row['exit'] == 1 and not row['original_delivery'] and not row['writers_cessation'], 'Failed lane promoted')
    checks = list((p.OUTPUT / 'final-checks').glob('*.json'))
    p.need({file.stem for file in checks} == {'offline', 'diff', 'format', 'docs', 'registry', 'contract'}, 'Final check set differs')
    for file in checks: p.need(load(file)['exit'] == 0, 'Final offline check failed')
    for name in ('syntax-native', 'syntax-boot', 'docs-final'):
        p.need(load(p.OUTPUT / (name + '.json'))['exit'] == 0, 'Final syntax/doc receipt absent or failed')
    return dict(environment_prepared=True, native_fd_helper_observed=True, native_control_overall_pass=False,
        first_cli_refusal_retained=True, first_property_refusal_retained=True, actual_start_ack_count=1,
        exact_stop_count=1, service_inactive_observed=True, original_delivery=False, writers_cessation=False,
        admission_allowed=False, server_operations=0, qualification_sha256=QUALIFICATION,
        sealer_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=p.ROOT, text=True).strip(),
        first_prepared_tar_sha256=p.sha(p.OUTPUT / 'debian-prepared.tar'),
        original_rootfs_sha256=p.sha(p.OUTPUT / 'debian-original.wsl'), prepared_rootfs_sha256=prepared['prepared_sha256'])


def run():
    result = audit()
    p.need(not PACKET.exists(), 'Never overwrite or reseal packet')
    PACKET.mkdir(parents=True)
    # Intentionally omit rootfs tar/gzip and the VHDX; retain their digests.
    for source, prefix in ((p.OUTPUT, 'environment'),
            (p.ROOT / 'target/held-native-fd-20261022', 'native-first'),
            (p.ROOT / 'target/held-native-fd-v2-20261022', 'native-second'),
            (p.ROOT / 'target/held-native-fd-cli-regression-20261022', 'regressions')):
        for file in sorted(source.rglob('*')):
            if file.is_file() and file.suffix not in ('.tar', '.wsl'):
                destination = PACKET / prefix / file.relative_to(source)
                destination.parent.mkdir(parents=True, exist_ok=True); shutil.copyfile(file, destination)
    for file in Path(__file__).parent.iterdir():
        if file.is_file():
            dest = PACKET / 'current-source' / file.name; dest.parent.mkdir(exist_ok=True); shutil.copyfile(file, dest)
    for commit in ('8b2ceb25', '4befa365', 'dde2cbde', 'f226d362'):
        for name in ('prepare.py', 'native_fd.py', 'native_fd.sh'):
            path = 'tools/held-wsl-native-074/' + name
            child = subprocess.run(['git', 'show', commit + ':' + path], capture_output=True, cwd=p.ROOT)
            if child.returncode == 0:
                dest = PACKET / 'committed-source' / commit / name; dest.parent.mkdir(parents=True, exist_ok=True)
                with dest.open('xb') as output: output.write(child.stdout)
    p.publish(PACKET / 'audit.json', result)
    files = [dict(path=file.relative_to(PACKET).as_posix(), bytes=file.stat().st_size, sha256=p.sha(file))
             for file in sorted(PACKET.rglob('*')) if file.is_file()]
    p.publish(PACKET / 'inventory.json', dict(files=files))
    print(json.dumps(dict(members=len(files), inventory_sha256=p.sha(PACKET / 'inventory.json'), **result), sort_keys=True))


if __name__ == '__main__': run()
