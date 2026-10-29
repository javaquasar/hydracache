"""Archive the first source-only V7 capture. No replay, subprocess or pin refresh."""
import json
from pathlib import Path

import package_effects_review_v7 as review


def retain(root):
    capture = root / 'target/package-review-v7-20261026/syntax-data-first'
    raw = (capture / 'result.json').read_bytes()
    review.require(review.sha(raw) == 'f83be32c93a8d52ac397f5a339d3ba997c1eac81e2a66c89ac89ee2b3804ed8f',
                   'first syntax bytes drift')
    document = review.decode(raw)
    gate = review.source_review(root, document)
    receipt = review.decode((capture / 'receipt.json').read_bytes())
    review.require(receipt['rounds'] == 2 and receipt['source_unchanged'] and receipt['context_closed'],
                   'first capture not closed')
    members, objects = {}, {}

    def add(name, body):
        review.require(name not in members, 'duplicate retained member')
        digest = review.sha(body)
        members[name] = dict(sha256=digest, bytes=len(body))
        objects[digest] = body

    for path in sorted(capture.iterdir()):
        review.require(path.is_file(), 'unexpected capture subtree')
        add('syntax/' + path.name, path.read_bytes())
    for label in ('parser-build', 'syntax-first'):
        folder = capture.parent / label
        result = review.decode((folder / 'result.json').read_bytes())
        review.require(result['exit'] == 0 and result['outer_refusal'] is None and
                       result['source_unchanged'] and result['commit_unchanged'], 'capture/build refused')
        for path in sorted(folder.rglob('*')):
            if path.is_file():
                add('checks/' + label + '/' + path.relative_to(folder).as_posix(), path.read_bytes())
    for name in sorted(set(gate['sources']) | set(gate['auxiliary'])):
        add('fixture/' + name, review.read_inside(root, name))
    for name in (review.PREVIOUS_SYNTAX, review.PREVIOUS_PACKET + '/index.json',
                 review.PREVIOUS_PACKET + '/objects/' + review.PREVIOUS_GATE_SHA256):
        add('fixture/' + name, review.read_inside(root, name))
    add('review/gate.json', (json.dumps(gate, sort_keys=True, indent=2) + '\n').encode())
    for name in ('run-xtask-package-780-20261024.py', 'run-source-step-20261024.py',
                 'run-package-review-v7-20261026.py', 'capture-package-review-v7-20261026.py'):
        add('coordinators/' + name, (root / 'target' / name).read_bytes())
    packet = root / 'docs/testing/performance/0.74/local-runs/package-review-v7-20261026'
    review.require({p.name for p in packet.iterdir()} == {'.gitattributes'}, 'append-only packet destination')
    (packet / 'objects').mkdir()
    for digest, body in objects.items():
        with (packet / 'objects' / digest).open('xb') as stream:
            stream.write(body)
    index = dict(schema='hc074-package-review-v7-evidence-v1', members=members,
                 source_files=372, syntax_sites=25988, targets=147,
                 canonical_ast_sha256=review.REVIEWED_AST_SHA256,
                 source_only=True, archived_code_executed=False, package_dispatched=False, admission=False)
    with (packet / 'index.json').open('x', encoding='utf-8', newline='\n') as stream:
        json.dump(index, stream, sort_keys=True, indent=2)
        stream.write('\n')
    print(json.dumps(dict(index_sha256=review.sha((packet / 'index.json').read_bytes()),
                          members=len(members), objects=len(objects))))


if __name__ == '__main__':
    retain(Path(__file__).resolve().parents[2])
