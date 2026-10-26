"""Verify the first V7 disk-floor refusal as data; no Git/process/archive execution."""
import hashlib
import json
from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
INDEX_SHA256 = '3f2605d66911660ec42d9da5bfd8614a8e507959c7f38473bbeaaea1b33be651'
SOURCE = '1d1658f4e26ff7eeceb07dac7c7ab3ab2c5a65d9'
ARGV = ['cargo', '+1.94.0', 'test', '--offline', '--locked', '--no-fail-fast',
        '-p', 'xtask', '--lib', '--tests', '--', '--test-threads=4']


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, 'duplicate JSON key')
            result[key] = value
        return result
    return json.loads(raw, object_pairs_hook=pairs)


def check(load=None):
    load = load or (lambda name: (HERE / name).read_bytes())
    raw = load('index.json')
    require(sha(raw) == INDEX_SHA256, 'index identity differs')
    index = decode(raw)
    require(index['source_commit'] == SOURCE and len(index['members']) == 57, 'receipt membership differs')
    require(all(index[k] is False for k in ('package_pass', 'automatic_retry', 'qualification', 'server')),
            'refusal promoted')
    for row in index['members'].values():
        require(re.fullmatch('[a-f0-9]{64}', row['sha256']) and type(row['bytes']) is int and row['bytes'] >= 0,
                'invalid object pointer')
        body = load('objects/' + row['sha256'])
        require(len(body) == row['bytes'] and sha(body) == row['sha256'], 'object identity differs')

    def member(name):
        return load('objects/' + index['members'][name]['sha256'])

    def data(name):
        return decode(member(name))

    gate = data('coordinator/full-first/gate.json')
    require(gate == data('coordinator/preflight-first/gate.json'), 'preflight gate changed')
    require(gate['head'] == SOURCE and gate['source_effect_review_complete'] is True
            and (len(gate['sources']), len(gate['auxiliary']), len(gate['targets'])) == (372, 51, 147),
            'review scope differs')
    require(all(gate[k] is False for k in ('package_pass', 'package_dispatched', 'formal_whole_program_proof',
                'linux_or_all_features_cleared', 'original_execution_or_product_or_qualification_admission')),
            'source review promoted')
    intent = data('package/intent.json')
    dispatch = data('coordinator/full-first/dispatch.json')
    require(intent['argv'] == dispatch['argv'] == ARGV and intent['head'] == SOURCE
            and intent['bound_seconds'] == 780 and intent['disk_floor_bytes'] == 4 * 1024 ** 3,
            'original command or limits differ')
    outer = data('checks/full-package-first/result.json')
    before = data('checks/full-package-first/intent.json')
    require(outer['exit'] == 1 and outer['outer_refusal'] == 'disk-floor'
            and outer['free_after'] < 4 * 1024 ** 3, 'first resource refusal differs')
    require(outer['source_unchanged'] is True and outer['commit_unchanged'] is True
            and outer['source_after'] == before['source'] and before['source_commit'] == SOURCE,
            'source/HEAD preservation differs')
    require('package/result.json' not in index['members'], 'missing inner receipt synthesized')
    require(member('package/stdout.log') == b'', 'unrecorded test output')
    stderr = member('package/stderr.log').decode()
    require(not any(line.strip().startswith(('Finished ', 'Running ')) for line in stderr.splitlines()),
            'compile/test stage differs')
    require(member('package/owned-cleanup.log') and member('checks/full-package-first/owned-cleanup.log'),
            'cleanup race evidence missing')
    accounting = data('derived/accounting.json')
    require(accounting['result_kind'] == 'build-aborted-at-disk-floor' and accounting['package_pass'] is False
            and accounting['completed_targets'] == 0 and accounting['expected_targets'] == 147
            and accounting['passed'] is None and accounting['failed'] is None
            and accounting['test_verdicts_present'] is False and accounting['original_inner_result_missing'] is True
            and accounting['strict_parser_executed'] is False
            and accounting['unstarted_targets'] == gate['targets'], 'invented test assessment')
    post = data('checks/postflight-first/stdout.log')
    require(post['source_review'] == {k: v for k, v in gate.items() if k not in ('environment', 'head')}
            and post['environment'] == gate['environment'] and post['head'] == SOURCE, 'postflight differs')
    result = data('checks/postflight-first/result.json')
    require(result['exit'] == 0 and result['outer_refusal'] is None
            and result['source_unchanged'] is True and result['commit_unchanged'] is True, 'postflight refused')
    return dict(verified=True, package_pass=False, test_verdicts_present=False,
                source_and_head_preserved=True, inner_result_synthesized=False,
                archived_code_executed=False, package_replayed=False, admission=False)


if __name__ == '__main__':
    print(json.dumps(check(), sort_keys=True))
