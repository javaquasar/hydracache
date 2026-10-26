"""Offline packet integrity and exact CI verdicts; never host admission."""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent


def check(root=HERE):
    root = Path(root)
    raw = (root/'index.json').read_bytes()
    if hashlib.sha256(raw).hexdigest() != '3fca8292fe4aa75a00bf327b264e75ca11293f595fade83d56f7418274c8da82':
        raise ValueError('index differs')
    index = json.loads(raw)
    if any(index[k] is not False for k in ('performance_claim', 'freeze_c74_allowed',
            'package_pass', 'production_original_delivery', 'qualification_allowed')):
        raise ValueError('packet promoted')
    members = index['members']
    for entry in members.values():
        data = (root/'objects'/entry['sha256']).read_bytes()
        if len(data) != entry['bytes'] or hashlib.sha256(data).hexdigest() != entry['sha256']:
            raise ValueError('object differs')
    def read(name):
        return (root/'objects'/members[name]['sha256']).read_bytes()
    helper = members['source/scripts/ci/watchdog_linux_group.py']['sha256']
    for phase, count, code in (('baseline', 4, 1), ('candidate', 4, 0), ('candidate-v2', 4, 0), ('extended-v2', 5, 0)):
        prefix = 'native/linux-ci-group-'+phase+'/'
        remote = json.loads(read(prefix+'stdout.log'))
        if json.loads(read(prefix+'result.json'))['ssh_exit'] != 0 or remote['exit'] != code:
            raise ValueError('native verdict differs')
        if f'Ran {count} tests' not in remote['stderr'] or remote['before'] != remote['after']:
            raise ValueError('native control or supervisor differs')
        if any(remote[k] is not False for k in ('services_changed', 'clocks_changed', 'installation', 'product_workload')):
            raise ValueError('native scope differs')
        if phase in ('candidate-v2', 'extended-v2') and remote['files']['watchdog_linux_group.py'] != helper:
            raise ValueError('accepted helper source differs')
    for label in ('linux-watchdog-models-portable-v3', 'linux-watchdog-existing-controls',
                  'linux-watchdog-ci-regression', 'linux-watchdog-check', 'linux-watchdog-clippy',
                  'linux-watchdog-windows-native-cli'):
        verdict = json.loads(read('checks/'+label+'/result.json'))
        if (verdict['exit'] != 0 or verdict['outer_refusal'] is not None or
                verdict['source_unchanged'] is not True or verdict['head_unchanged'] is not True):
            raise ValueError('focused verdict differs')
    return dict(members=len(members), integrity=True, package_pass=False, server_admitted=False)


if __name__ == '__main__':
    print(check())
