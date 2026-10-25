"""Replay the retained first package verdict offline, without Cargo or subprocesses."""
import hashlib
import importlib.util
import json
from pathlib import Path

INDEX_SHA256 = '0b0b568cc495e0a24f21f6684dbc6d471e3e3d1c1240fce002958975ebd31a9a'
ROOT = Path(__file__).resolve().parents[6]
PARSER = 'tools/xtask-package-attribution-074/review.py'


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def verify(folder):
    raw = (folder / 'index.json').read_bytes()
    require(sha(raw) == INDEX_SHA256, 'fixed first index differs')
    index = json.loads(raw)
    bodies = {}
    for name, row in index['members'].items():
        require(len(row['sha256']) == 64 and all(c in '0123456789abcdef' for c in row['sha256']), 'object digest')
        body = (folder / 'objects' / row['sha256']).read_bytes()
        require(sha(body) == row['sha256'] and len(body) == row['bytes'], 'raw member differs: ' + name)
        bodies[name] = body
    require(len(bodies) == 30, 'fixed member inventory differs')
    parser_path = ROOT / PARSER
    require(parser_path.read_bytes() == bodies['source/' + PARSER], 'current offline parser drift')
    spec = importlib.util.spec_from_file_location('hc074_pinned_package_transcript', parser_path)
    parser = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(parser)
    intent = json.loads(bodies['package/intent.json'])
    result = json.loads(bodies['package/result.json'])
    gate = json.loads(bodies['preflight/gate.json'])
    require(intent['head'] == index['source_commit'] == gate['head'] ==
            'b0f627fec0d592485233b527c61014128b4d689a', 'original source commit differs')
    require(result['source_unchanged'] is True and result['head_unchanged'] is True and
            result['source_after'] == intent['source'], 'source/HEAD changed')
    require(intent['argv'] == parser.ARGV and intent['bound_seconds'] == 780 and intent['retry'] is False,
            'original command/window changed')
    require(len(gate['sources']) == 344 and len(gate['auxiliary']) == 29 and
            gate['source_effect_review_complete'] is True and gate['package_pass'] is False,
            'review scope differs')
    targets = gate['targets']
    require(len(set(targets)) == len(targets) == 138, 'full catalog differs')
    coordinator = json.loads(bodies['preflight/coordinator.json'])
    for path, expected in coordinator['wrappers'].items():
        require(sha(bodies['coordinators/' + Path(path).name]) == expected, 'original wrapper drift')
    computed = parser.parse_transcript(bodies['package/stdout.log'].decode('utf-8'),
        bodies['package/stderr.log'].decode('utf-8'), targets, result['exit'],
        result['outer_refusal'], intent['bound_seconds'])
    require(computed == json.loads(bodies['analysis-first.json']), 'derived result differs from raw transcript')
    require(computed['complete'] is False and computed['package_pass'] is False and
            index['package_complete'] is False and index['package_pass'] is False and
            index['product_or_qualification_admission'] is False, 'incomplete first result relabeled')
    return dict(source=index['source_commit'], package_pass=False,
                completed_targets=computed['completed_targets'], expected_targets=computed['expected_targets'],
                passed_in_completed=computed['passed'], failed_in_completed=computed['failed'],
                observed_passed=computed['individual_passed'], observed_failed=computed['individual_failed'],
                incomplete_target=computed['incomplete_target']['target'],
                incomplete_observations=len(computed['incomplete_target']['cases']),
                unstarted_targets=len(computed['unstarted_targets']), outer_refusal=computed['outer_refusal'])


if __name__ == '__main__':
    print(json.dumps(verify(Path(__file__).resolve().parent), indent=2, sort_keys=True))
