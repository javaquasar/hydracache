"""Exact terminal source deltas from independent fixed Git producers."""
from pathlib import Path
import argparse
import difflib
import hashlib
import json
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SOURCE = '33c54b0cda8c330b115009cdf092ad002077cedf'
BASE = 'bf352a1dfbe3040af9ce058a5d42efe55202db62'
TERMINAL = 'a67a5299ab335424e9b11330dc06bd749ea9b859'
MAIN_PREVIOUS = '37db65f68e67ed7c0ee0f1cc84fed1c30bf9822a'
PRODUCERS = {
    'crates/hydracache-redis-compat/Cargo.toml': TERMINAL,
    'crates/hydracache-redis-compat/README.md': TERMINAL,
    'crates/hydracache-redis-compat/src/get_response_owner_074.rs': TERMINAL,
    'crates/hydracache-redis-compat/src/lib.rs': TERMINAL,
    'crates/xtask/tests/performance_contract_074.rs': TERMINAL,
    'docs/testing/performance/0.74/proposal-registry.toml': TERMINAL,
    'tools/get-owner-scheduled-controls-074/Cargo.toml': '8c5d4810837f91483625dff119edd5513c605e04',
    'tools/get-owner-scheduled-controls-074/README.md': TERMINAL,
    'tools/get-owner-scheduled-controls-074/src/bin/timing_controls.rs': 'be0499a8cb0a2ad0b7328d6ac0cf6227114cebcc',
    'tools/get-owner-scheduled-controls-074/src/security.rs': '5a3785a6a4de12cbfd8ee58470b333ac7677fda4',
    'tools/get-owner-scheduled-controls-074/src/timing.rs': 'be0499a8cb0a2ad0b7328d6ac0cf6227114cebcc',
    'tools/get-owner-scheduled-controls-074/tests/timing_cli.rs': '5a3785a6a4de12cbfd8ee58470b333ac7677fda4',
}


def frozen(revision, path):
    return subprocess.check_output(['git', '--no-replace-objects', 'show', revision + ':' + path],
                                   cwd=ROOT).decode().replace('\r\n', '\n')


def sha(text):
    return hashlib.sha256(text.encode()).hexdigest()


def description():
    rows = []
    for path, revision in PRODUCERS.items():
        previous_revision = MAIN_PREVIOUS if path == 'crates/xtask/tests/performance_contract_074.rs' else SOURCE
        previous, current = frozen(previous_revision, path), frozen(revision, path)
        assert current == frozen(BASE, path) and previous != current
        before, after = previous.splitlines(keepends=True), current.splitlines(keepends=True)
        patches = []
        for kind, i, end_i, j, end_j in difflib.SequenceMatcher(a=before, b=after, autojunk=False).get_opcodes():
            if kind != 'equal':
                patches.append(dict(offset=len(''.join(after[:j]).encode()),
                                    current=''.join(after[j:end_j]), previous=''.join(before[i:end_i])))
        rows.append(dict(path=path, revision=revision, previous_revision=previous_revision,
                         current_sha256=sha(current), previous_sha256=sha(previous), patches=patches))
    path = 'crates/xtask/tests/current_tree_input_audit_074.rs'
    old = frozen(BASE, path)
    edits = [dict(before='current_inputs::' + name, after='frontier::' + name, count=1)
             for name in ['known_addition(path)', 'require_addition(path)', 'read(path)']]
    new = old
    for edit in edits:
        assert new.count(edit['before']) == edit['count']
        new = new.replace(edit['before'], edit['after'])
    append = '\n#[path = "../../../tools/current-frontier-074/reader.rs"]\nmod frontier;\n'
    new += append
    return dict(schema='hc074-current-frontier-v1', source=SOURCE, baseline=BASE, rows=rows,
                audit=dict(path=path, before_sha256=sha(old), after_sha256=sha(new), edits=edits, append=append))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    args = parser.parse_args()
    data = (json.dumps(description(), indent=2) + '\n').encode()
    path = Path(__file__).with_name('rules.json')
    if args.write:
        with path.open('xb') as output:
            output.write(data)
    else:
        assert path.read_bytes() == data, 'fixed source description differs'
    print(hashlib.sha256(data).hexdigest())
