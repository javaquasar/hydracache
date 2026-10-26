"""Offline paired-source, graph, verdict and ownership-boundary verification."""
import ast
import hashlib
import json
from pathlib import Path
import re
import tomllib

HERE = Path(__file__).resolve().parent
INDEX_SHA256 = '2f984fa20ea11841a8b69a961cbdea872238b97baa00f47fc87c70057879e83e'


def check(load=None):
    load = load or (lambda name: (HERE/name).read_bytes())
    raw = load('index.json')
    if hashlib.sha256(raw).hexdigest() != INDEX_SHA256:
        raise ValueError('index differs')
    index = json.loads(raw)
    for row in index['members'].values():
        raw = load('objects/'+row['sha256'])
        if len(raw) != row['bytes'] or hashlib.sha256(raw).hexdigest() != row['sha256']:
            raise ValueError('object differs')
    def read(name):
        return load('objects/'+index['members'][name]['sha256'])
    def text(name):
        return read(name).decode().replace('\r\n', '\n')
    for field in ('full_graph_io_safety_proved','original_checks_or_deadlines_changed',
            'roots_candidate_accepted','policy_pass','full_package_pass',
            'original_execution_proved','server','release_admitted'):
        if index[field] is not False:
            raise ValueError('packet promoted')
    baseline = 'checks/baseline/inputs/'
    fixture = 'tools/long-run-supervisor-074/src/diagnostic_manager_alarm_tests.rs'
    for phase in ('paired-baseline','candidate-alarm','candidate-channel','msrv-alarm','msrv-channel'):
        if read('checks/'+phase+'/inputs/'+fixture) != read('source/'+fixture):
            raise ValueError('paired oracle differs')
    before = tomllib.loads(text(baseline+'Cargo.lock'))
    after = tomllib.loads(text('source/Cargo.lock'))
    old = next(p for p in before['package'] if p['name'] == 'tokio')
    new = next(p for p in after['package'] if p['name'] == 'tokio')
    if (old['version'],new['version']) != ('1.52.3','1.53.3'):
        raise ValueError('version differs')
    old.update(version=new['version'],checksum=new['checksum'])
    if before != after or index['locked_nodes_changed'] != 1:
        raise ValueError('unrelated lock delta')
    old = tomllib.loads(text(baseline+'Cargo.toml'))
    new = tomllib.loads(text('source/Cargo.toml'))
    if old['workspace']['dependencies']['tokio']['version'] != '1.52.3':
        raise ValueError('old floor differs')
    old['workspace']['dependencies']['tokio']['version'] = '1.53.3'
    if old != new or new['workspace']['package']['rust-version'] != '1.88':
        raise ValueError('manifest or MSRV differs')
    channel = 'tools/long-run-supervisor-074/src/diagnostic_manager_channel.rs'
    source = text('source/'+channel)
    start = source.index('// Private ownership boundary;')
    end = source.index('impl Operation {', start)
    tree = ast.parse(text('coordinator/audit-tokio-security-20261026.py'))
    expected = next(ast.literal_eval(row.value) for row in tree.body
        if isinstance(row, ast.Assign) and any(isinstance(t,ast.Name) and t.id == 'expected' for t in row.targets))
    if source[start:end] != expected:
        raise ValueError('owned registration contract differs')
    restored = (source[:start]+source[end:]).replace(
        'let alarm = register_operation_alarm(&self.file)?;',
        'let alarm = io(tokio::io::unix::AsyncFd::new(io(self.file.try_clone())?))?;')
    added = '\n#[cfg(test)]\n#[path = "diagnostic_manager_alarm_tests.rs"]\nmod alarm_tests;\n'
    if restored.count(added) != 1 or restored.replace(added, '') != text(baseline+channel):
        raise ValueError('channel predicates or ordering differ')
    for name in ('tools/long-run-supervisor-074/src/diagnostic_manager_channel_kernel.rs',
            'deny.toml','docs/testing/performance/0.74/qualification-manifest.toml'):
        if read('source/'+name) != read(baseline+name):
            raise ValueError('protected input differs')
    qual = read('source/docs/testing/performance/0.74/qualification-manifest.toml')
    if hashlib.sha256(qual).hexdigest() != '11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc':
        raise ValueError('qualification differs')
    phases = {n.split('/')[1] for n in index['members'] if n.startswith('checks/')}
    if len(phases) != 19:
        raise ValueError('focused phase membership differs')
    counts = {}
    for phase in phases:
        result = json.loads(read('checks/'+phase+'/result.json'))
        if (result['exit'] != (1 if phase == 'policy' else 0) or result['outer_refusal'] is not None
                or result['source_contract'] is not True or result['commit_unchanged'] is not True):
            raise ValueError('first focused result differs')
        rows = re.findall(r'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;', text('checks/'+phase+'/stdout.log'))
        if rows:
            counts[phase] = dict(targets=len(rows),totals=[sum(int(r[i]) for r in rows) for i in range(5)])
    if counts != index['counts']:
        raise ValueError('focused counts differ')
    for phase in ('paired-baseline','candidate-alarm','msrv-alarm'):
        if counts[phase]['totals'][:4] != [4,0,0,0]:
            raise ValueError('alarm cohort differs')
    for phase in ('baseline','candidate-channel','msrv-channel'):
        if counts[phase]['totals'][:4] != [9,0,2,0]:
            raise ValueError('channel cohort differs')
    if counts['resp']['totals'] != [183,0,23,0,0] or counts['server']['totals'] != [15,0,0,0,0] or counts['msrv-tls']['totals'] != [5,0,0,0,0]:
        raise ValueError('compatibility cohort differs')
    ids = sorted(set(re.findall(r'ID: (RUSTSEC-\d{4}-\d{4})', text('checks/policy/stderr.log'))))
    if ids != index['remaining_advisories'] or len(ids) != 9 or 'RUSTSEC-2026-0386' in ids:
        raise ValueError('remaining findings differ')
    return dict(members=len(index['members']),integrity=True,same_paired_oracle=True,
                policy_pass=False,full_package_pass=False,release_admitted=False)


if __name__ == '__main__':
    print(check())
