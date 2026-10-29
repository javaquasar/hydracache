"""Read the first V8 receipt as data; never dispatch or execute archived code."""
import hashlib
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACKET = 'docs/testing/performance/0.74/local-runs/package-closure-v8-20261027'
INDEX = 'ad41027076d5cc20e52267976e05b507bd9b25e35fc0e71b08adc280be062432'
HEAD = '0a6f8c8e6d6467777d519bd6799800bf694142f3'
PARSER = '0229d4067fffd0d0b5acee7d864efc64aadfb201de1ea3f17944a2f8dddf3c4d'
ARGV = ['cargo', '+1.94.0', 'test', '--offline', '--locked', '--no-fail-fast',
        '-p', 'xtask', '--lib', '--tests', '--', '--test-threads=4']


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def load_packet(index_raw, read_object):
    require(sha(index_raw) == INDEX, 'first V8 index drift')
    index = json.loads(index_raw)
    require(index['source_commit'] == HEAD and len(index['members']) == 55,
            'first V8 identity or member count drift')
    members = {}
    for name, row in index['members'].items():
        raw = read_object(row['sha256'])
        require(len(raw) == row['bytes'] and sha(raw) == row['sha256'],
                f'first V8 object drift: {name}')
        members[name] = raw
    return members


def assess(members, parse_transcript):
    """Compare retained claims with the unchanged current, byte-pinned parser."""
    read = lambda name: json.loads(members[name])
    gate = read('coordinator/full-first/gate.json')
    intent, result = read('package/intent.json'), read('package/result.json')
    require(gate['head'] == intent['head'] == HEAD, 'source commit drift')
    require(intent['argv'] == ARGV and intent['bound_seconds'] == 780
            and intent['retry'] is False, 'original command/window/retry drift')
    require(result['source_unchanged'] is True and result['head_unchanged'] is True
            and result['source_after'] == intent['source'], 'source observation drift')
    require(result['admission'] is False and result['qualification'] is False,
            'unexpected execution admission')
    post = read('checks/postflight-first/stdout.log')
    require(post['head'] == HEAD and post['environment'] == gate['environment']
            and post['source_review'] == {k: v for k, v in gate.items()
                                         if k not in ('head', 'environment')},
            'source/effect/environment postflight drift')
    for phase in ('preflight-first', 'full-package-first', 'postflight-first'):
        observation = read(f'checks/{phase}/result.json')
        require(observation['source_unchanged'] is True
                and observation['commit_unchanged'] is True, 'outer source drift')
    actual = parse_transcript(members['package/stdout.log'].decode('utf-8'),
                              members['package/stderr.log'].decode('utf-8'),
                              gate['targets'], result['exit'], result['outer_refusal'], 780)
    require(actual == read('derived/accounting.json'), 'retained accounting drift')
    require((actual['expected_targets'], actual['completed_targets'],
             actual['passed'], actual['failed'], actual['ignored']) == (148, 81, 456, 36, 0)
            and len(actual['unstarted_targets']) == 67
            and actual['incomplete_target'] is None
            and actual['outer_refusal'] == 'outer-timeout'
            and actual['complete'] is False and actual['package_pass'] is False,
            'first refusal cannot become a complete package pass')
    return dict(receipt_integrity=True, package_pass=False, complete=False,
                expected_targets=148, completed_targets=81, passed=456, failed=36,
                unstarted_targets=67, workload_rerun=False, release_admitted=False)


def current_parser(root=ROOT):
    path = root / 'tools/xtask-package-attribution-074/review.py'
    require(sha(path.read_bytes()) == PARSER, 'current transcript parser drift')
    spec = importlib.util.spec_from_file_location('hc074_checked_v8_result_parser', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.parse_transcript


def verify(root=ROOT):
    folder = root / PACKET
    members = load_packet((folder / 'index.json').read_bytes(),
                          lambda digest: (folder / 'objects' / digest).read_bytes())
    return assess(members, current_parser(root))


if __name__ == '__main__':
    print(json.dumps(verify(), sort_keys=True))
