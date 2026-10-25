"""Retain the fixed source-only capture as data, never executing archived code."""
import json
from pathlib import Path

import package_effects_review_v4 as review


def retain(root):
    capture = root / 'target/package-syntax-v4-closure-20261025'
    raw = (capture / 'result.json').read_bytes()
    review.require(review.sha(raw) == 'd674bfca2dffa3f887735677f1e6aabb88b0440522e89f43757774881c1ca122',
                   'first syntax capture differs')
    document = json.loads(raw)
    gate = review.source_review(root, document)
    review.require(json.loads((capture / 'edges-1.json').read_bytes()) == dict(additions={}, unresolved=[]),
                   'unclosed first inventory')
    members, objects = {}, {}

    def add(name, body):
        review.require(name not in members, 'duplicate member')
        digest = review.sha(body)
        members[name] = dict(sha256=digest, bytes=len(body))
        objects[digest] = body

    for path in sorted(capture.iterdir()):
        review.require(path.is_file(), 'unexpected capture subtree')
        add('syntax/' + path.name, path.read_bytes())
    for label in ('package-closure-v4-syntax-build-first', 'package-closure-v4-syntax-capture-first'):
        folder = root / 'target/source-continuation-20261024' / label
        result = json.loads((folder / 'result.json').read_bytes())
        review.require(result['exit'] == 0 and result['outer_refusal'] is None and
                       result['source_unchanged'] and result['head_unchanged'], 'capture/build failed')
        for path in sorted(folder.iterdir()):
            review.require(path.is_file(), 'unexpected check subtree')
            add('checks/' + label + '/' + path.name, path.read_bytes())
    for name in sorted(set(gate['sources']) | set(gate['auxiliary'])):
        add('fixture/' + name, (root / name).read_bytes())
    old_folder = root / review.PREVIOUS_PACKET
    old_raw = (old_folder / 'index.json').read_bytes()
    old_object = json.loads(old_raw)['members']['syntax/result.json']['sha256']
    add('fixture/' + review.PREVIOUS_PACKET + '/index.json', old_raw)
    add('fixture/' + review.PREVIOUS_PACKET + '/objects/' + old_object,
        (old_folder / 'objects' / old_object).read_bytes())
    for name in ('run-xtask-package-780-20261024.py', 'run-source-step-20261024.py'):
        add('wrappers/' + name, (root / 'target' / name).read_bytes())
    packet = root / 'docs/testing/performance/0.74/local-runs/package-effect-syntax-v4-closure-20261025'
    review.require(packet.is_dir() and {p.name for p in packet.iterdir()} == {'.gitattributes'},
                   'packet destination must contain only the reviewed attributes scaffold')
    (packet / 'objects').mkdir()
    for digest, body in objects.items():
        with (packet / 'objects' / digest).open('xb') as stream:
            stream.write(body)
    index = dict(schema='hc074-package-effect-syntax-v4-closure-evidence-v1', members=members,
                 canonical_ast_sha256=review.REVIEWED_AST_SHA256, source_files=344,
                 syntax_sites=sum(len(row['sites']) for row in document['sources']),
                 source_only=True, archived_code_executed=False, package_replayed=False, admission=False)
    with (packet / 'index.json').open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(index, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(dict(index_sha256=review.sha((packet / 'index.json').read_bytes()),
                          members=len(members), objects=len(objects))))


if __name__ == '__main__':
    retain(Path(__file__).resolve().parents[2])
