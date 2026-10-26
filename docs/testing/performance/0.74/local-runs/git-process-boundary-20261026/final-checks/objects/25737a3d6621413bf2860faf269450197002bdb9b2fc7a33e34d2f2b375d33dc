"""Offline bytes and identity join; never imports/runs the retained control."""
import hashlib
import json
from pathlib import Path

INDEX = 'c59c64b3f666377ad1098c1795ceead8583f0f1a0d96f03efda1d5f41ad741c5'
SOURCE = '3807c4749873d9a3e9495c755bbf3ef1f73f9c7b'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def require(value, message):
    if not value:
        raise ValueError(message)


def verify(folder):
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == INDEX, 'index differs')
    index = json.loads(raw)
    require(index['source_commit'] == SOURCE and index['native_control_attempts'] == 1 and
            all(index[key] is False for key in ('performance_cohort', 'package_pass', 'server', 'product_admission')),
            'index scope differs')
    members = {}
    for name, row in index['members'].items():
        body = (folder / 'objects' / row['sha256']).read_bytes()
        require(len(body) == row['bytes'] and sha(body) == row['sha256'], 'raw member differs: ' + name)
        members[name] = body
    require(len(members) == 75, 'member count differs')
    labels = sorted(name.split('/')[1] for name in members
                    if name.startswith('checks/') and name.endswith('/result.json'))
    require(len(labels) == 14, 'check catalog differs')
    for label in labels:
        prefix = 'checks/' + label + '/'
        intent, result = (json.loads(members[prefix + name]) for name in ('intent.json', 'result.json'))
        require(result['exit'] == (1 if label == 'git-boundary-close-budget-first' else 0) and
                result['outer_refusal'] is None and result['source_unchanged'] is True and
                result['head_unchanged'] is True and intent['source'] == result['source_after'],
                'check result/source differs')
        require(intent['bound_seconds'] == 1200 and intent['disk_floor_bytes'] == 4 * 1024**3 and
                all(intent[key] is False for key in ('dependency_changes', 'observation_frequency_changed',
                    'services_or_clocks', 'server', 'original_owner', 'qualification', 'retry')), 'check scope differs')
    for label, count, ending in (('git-boundary-close-budget-first', 13, 'FAILED (failures=1)'),
                                 ('git-boundary-close-budget-fixed', 13, 'OK'),
                                 ('git-boundary-precommit-models', 60, 'OK')):
        stderr = members['checks/' + label + '/stderr.log'].decode()
        require('Ran ' + str(count) + ' tests' in stderr and stderr.rstrip().endswith(ending), 'model verdict differs')
    native_before = json.loads(members['checks/git-boundary-native-first/intent.json'])
    require(native_before['head'] == SOURCE and native_before['argv'] == [
        'C:/Python313/python.exe', 'tools/xtask-package-attribution-074/git_process_boundary.py', 'control', 'first'],
        'native command/source differs')
    for name, body in members.items():
        if name.startswith('source/'):
            require(sha(body) == native_before['source'][name[7:]], 'source copy differs')
    before = json.loads(members['checks/git-boundary-close-budget-first/intent.json'])
    for name in ('git_process_boundary.py', 'test_git_process_boundary.py'):
        require(sha(members['raw-before-close-budget/' + name]) ==
                before['source']['tools/xtask-package-attribution-074/' + name], 'raw first source differs')
    current = members['source/tools/xtask-package-attribution-074/git_process_boundary.py'].decode()
    for block in ("                require(clock() < discovery_deadline, 'late discovery witness')\n",
                  "            if time.monotonic() >= cleanup_deadline:\n                result['cleanup_errors'].append('late cleanup witness')\n"):
        require(current.count(block) == 1, 'fixed fresh budget check differs')
        current = current.replace(block, '')
    require(current.encode() == members['raw-before-close-budget/git_process_boundary.py'], 'bounded source delta differs')
    intent, result = (json.loads(members['native/' + name]) for name in ('intent.json', 'result.json'))
    require(intent['retry'] is False and intent['object_requests'] == 0 and
            result['passed'] is True and result['refusal'] is None and result['cleanup_errors'] == [] and
            result['exit_code'] == 0 and result['argv'] == intent['argv'] and
            result['image_pins_before'] == result['image_pins_after'] == intent['independent_images'], 'native verdict/pins differ')
    require(not members['native/stdout.log'] and not members['native/stderr.log'], 'nonempty native streams')
    require((result['run_seconds'], result['discovery_seconds'], result['cleanup_seconds']) == (10, 5, 5) and
            result['discovery_snapshots'] == 1 and result['object_requests'] == 0, 'native limits differ')
    identities = result['identities']
    require(len(identities) == 2 and identities[0]['pid'] != identities[1]['pid'] and
            [row['image'].lower().replace('/', '\\') for row in identities] ==
                [r'c:\program files\git\cmd\git.exe', r'c:\program files\git\mingw64\bin\git.exe'] and
            0 < identities[0]['creation_100ns'] <= identities[1]['creation_100ns'], 'held chain differs')
    for name, identity in zip(('root_samples', 'native_samples'), identities):
        first, last = result[name]
        require(first['terminal'] is False and first['exit_100ns'] is None and last['terminal'] is True and
                first['creation_100ns'] == last['creation_100ns'] == identity['creation_100ns'] and
                last['exit_100ns'] >= last['creation_100ns'], 'held live/terminal witness differs')
        require(all(last[key] >= first[key] >= 0 for key in ('kernel_100ns', 'user_100ns')) and
                all(last['io'][key] >= first['io'][key] >= 0 for key in first['io']), 'counter monotonicity differs')
    require(result['root_samples'][-1]['exit_100ns'] >= result['native_samples'][-1]['exit_100ns'], 'exit order differs')
    require(all(result[key] is False for key in ('all_descendants_accounted', 'mapped_image_attested',
                'original_helper_executed', 'package_pass', 'performance_cohort', 'product_admission')),
            'native scope promoted')
    require(result['analysis'] == dict(current_full_helper_cost_attributed=False,
        directly_created_counter_excludes_native=True, numeric_speedup_established=False,
        old_cohort_topology_attested=False, package_pass=False, two_distinct_held_processes=True), 'analysis differs')
    return dict(native_attempts=1, distinct_held_processes=2, object_requests=0, models=60,
                first_late_close_failure_preserved=True, directly_created_counter_excludes_native=True,
                performance_cohort=False, package_pass=False, workload_replayed=False)


if __name__ == '__main__':
    print(verify(Path(__file__).resolve().parent))
