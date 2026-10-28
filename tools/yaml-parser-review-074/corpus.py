"""Fixed review corpus; no edits to workflow files and no workflow execution."""
import hashlib
import json
from pathlib import Path
import random

SEED = 0x0741028


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def cases(root):
    rows = []
    for path in sorted((root/'.github/workflows').iterdir()):
        if path.suffix not in ('.yml', '.yaml'):
            continue
        if not path.is_file() or path.is_symlink() or path.is_junction():
            raise ValueError('workflow custody')
        raw = path.read_bytes()
        rows.append(dict(id=path.relative_to(root).as_posix(), text=raw.decode('utf-8'),
                         expect='valid', source_sha256=sha(raw)))
    fixtures = [
        ('empty', '', 'valid'), ('null', 'null\n', 'valid'),
        ('keys-and-scalars', 'on: true\nyes: no\nnull: ~\ntrue: false\n', 'valid'),
        ('null-keys', '? null\n: value\n', 'valid'),
        ('numeric-boundaries', '[0, -0, -0.0, 18446744073709551615, -9223372036854775808, 1.25, .nan, .inf, -.inf]\n', 'valid'),
        ('tagged', 'items: [!Thing {x: 1}, !!str true, !!int 12]\n', 'valid'),
        ('alias', 'a: &shared [one, two]\nb: *shared\n', 'valid'),
        ('merge', 'a: &base {runs-on: ubuntu-latest, timeout-minutes: 10}\nb: {<<: *base, name: check}\n', 'valid'),
        ('complex-key', '? [one, two]\n: {value: 3}\n', 'valid'),
        ('quoted-key', "'on': {push: {branches: [main]}}\n", 'valid'),
        ('multiline', 'run: |\n  echo "${{ github.ref }}"\n  echo finished\nfolded: >-\n  alpha\n  beta\n', 'valid'),
        ('crlf', 'on: push\r\njobs:\r\n  test:\r\n    runs-on: windows-latest\r\n', 'valid'),
        ('duplicate-key', 'jobs: 1\njobs: 2\n', 'invalid'),
        ('duplicate-nested-key', 'jobs: {test: 1, test: 2}\n', 'invalid'),
        ('duplicate-alias-key', 'a: &key x\n? *key\n: one\nx: two\n', 'invalid'),
        ('undefined-alias', 'jobs: *missing\n', 'invalid'),
        ('multiple-documents', '---\non: push\n---\non: pull_request\n', 'invalid'),
        ('truncated-flow', 'jobs: [one, two\n', 'invalid'),
        ('invalid-indentation', 'jobs:\n  test:\n runs-on: linux\n', 'invalid'),
        ('tab-indent', 'jobs:\n\ttest: value\n', 'invalid'),
        ('invalid-escape', 'value: "\\q"\n', 'invalid'),
        ('unterminated-quote', 'value: "unfinished\n', 'invalid'),
        ('raw-control', 'value: "a\x01b"\n', 'invalid'),
        ('two-tags', 'value: !first !second text\n', 'invalid'),
    ]
    for name, text, expectation in fixtures:
        rows.append(dict(id='fixture/'+name, text=text, expect=expectation))
    for depth in (32, 64, 127, 128, 129):
        rows.append(dict(id='boundary/depth-'+str(depth), text='['*depth+'0'+']'*depth+'\n', expect='compare'))
    rng = random.Random(SEED)
    tokens = ['on', 'true', 'false', 'null', '~', '08', '0x7f', '2026-10-28',
              '${{ github.ref }}', 'a: # b', 'line\nbreak', 'nul\0inside', 'русский', '😀']
    for index in range(48):
        token = rng.choice(tokens)
        value = json.dumps(token, ensure_ascii=False)
        timeout = rng.choice([0, 1, 10, 60, 360])
        rows.append(dict(id='seeded/'+str(index), expect='valid',
            text='on: [push, pull_request]\nenv:\n  VALUE: '+value+'\njobs:\n  check:\n    timeout-minutes: '+str(timeout)+'\n    runs-on: ubuntu-latest\n'))
    if not rows or len(rows) > 200 or len({r['id'] for r in rows}) != len(rows):
        raise ValueError('fixed corpus identity/count')
    return rows
