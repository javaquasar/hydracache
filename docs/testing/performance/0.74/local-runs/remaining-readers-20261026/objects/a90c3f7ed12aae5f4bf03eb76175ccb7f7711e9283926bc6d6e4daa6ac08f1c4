"""Exact caller constructions from a fixed clean predecessor, not observed approval."""
from pathlib import Path
import argparse
import difflib
import hashlib
import json
import subprocess

ROOT = Path(__file__).resolve().parents[2]
BASE = '3e7e0aa343ed51f99651761f9bb7b0b10efd359a'
PREFIX = 'crates/xtask/tests/'
FRESH = PREFIX + 'fresh_kernel_cost_074.rs'
SOURCE = 'tools/long-run-supervisor-074/src/diagnostic_kernel_origin.rs'
HOOK = '#[path = "support/current_tree_input.rs"]'
FACADE = '#[path = "../../../tools/remaining-readers-074/inputs.rs"]'
sha = lambda text: hashlib.sha256(text.encode()).hexdigest()


def frozen(path):
    return subprocess.check_output(['git', '--no-replace-objects', 'show', BASE + ':' + path],
                                   cwd=ROOT).decode().replace('\r\n', '\n')


def changes():
    fresh_before = '    guard::pid_path::custody_cost::document_scratch::normalize(path, &raw)'
    fresh_after = '    let raw = if path == "' + SOURCE + '" {\n        runtime::successor::project(path, raw)\n    } else {\n        raw\n    };\n' + fresh_before
    raw_before = 'fn raw(path: &str) -> String {\n    std::fs::read_to_string(runtime::root().join(path))\n        .unwrap()\n        .replace("\\r\\n", "\\n")\n}'
    raw_after = 'fn raw(path: &str) -> String {\n    let raw = std::fs::read_to_string(runtime::root().join(path))\n        .unwrap()\n        .replace("\\r\\n", "\\n");\n    if path == "crates/xtask/tests/current_tree_input_audit_074.rs" {\n        current_inputs::phase::previous(path, &raw).expect("exact next audit reader phase")\n    } else {\n        raw\n    }\n}'
    bootstrap = 'tools/bootstrap-doc-reader-074/reader.rs'
    hook = 'pub fn read(path: &str, raw: String) -> String {\n'
    return [
        (FRESH, [(fresh_before, fresh_after)], ''),
        (PREFIX + 'held_activation_ingress_074.rs', [(
            '    let raw = std::fs::read_to_string(root.join("docs/COMPAT.md"))',
            '    let raw = bootstrap_docs::read_to_string(root.join("docs/COMPAT.md"))')],
            '\n#[path = "../../../tools/bootstrap-doc-reader-074/io.rs"]\nmod bootstrap_docs;\n'),
        (PREFIX + 'current_tree_reader_074.rs', [(HOOK, FACADE)], ''),
        (PREFIX + 'current_tree_input_audit_074.rs', [(HOOK, FACADE)], ''),
        (PREFIX + 'current_frontier_074.rs', [(HOOK, FACADE), (raw_before, raw_after)], ''),
        (bootstrap, [(hook, hook +
            '    if path == "crates/xtask/tests/current_tree_reader_074.rs" {\n        return remaining_phase::previous(path, &raw)\n            .expect("exact next current-tree reader phase");\n    }\n')],
            '\n#[path = "../remaining-readers-074/contract.rs"]\nmod remaining_phase;\n'),
        (PREFIX + 'support/document_scratch_inputs.rs', [(
            'pub(crate) fn accepts(path: &str, raw: &str) -> bool {',
            'fn accepts_previous(path: &str, raw: &str) -> bool {')],
            '\n#[path = "../../../../tools/remaining-readers-074/contract.rs"]\nmod remaining_phase;\npub(crate) fn accepts(path: &str, raw: &str) -> bool {\n    if path == remaining_phase::FRESH {\n        return remaining_phase::previous(path, raw)\n            .is_some_and(|previous| accepts_previous(path, &previous));\n    }\n    accepts_previous(path, raw)\n}\n')]


def rows():
    result = []
    for path, replacements, append in changes():
        old = frozen(path)
        current = old
        edits = []
        for before, after in replacements:
            assert current.count(before) == 1
            current = current.replace(before, after)
            edits.append(dict(before=before, after=after, count=1))
        current += append
        result.append((dict(path=path, before_sha256=sha(old), after_sha256=sha(current),
                            edits=edits, append=append), old, current))
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    parser.add_argument('--patch', action='store_true')
    parser.add_argument('--emit', action='store_true')
    args = parser.parse_args()
    planned = rows()
    if args.patch:
        assert not args.write
        print('*** Begin Patch')
        for row, old, current in planned:
            path = ROOT / row['path']
            assert path.read_text(encoding='utf-8') == old, 'foreign caller changes: ' + row['path']
            print('*** Update File: ' + path.as_posix())
            before, after = old.splitlines(), current.splitlines()
            for group in difflib.SequenceMatcher(a=before, b=after, autojunk=False).get_grouped_opcodes(3):
                print('@@')
                for kind, i, end_i, j, end_j in group:
                    if kind == 'equal':
                        for line in before[i:end_i]: print(' ' + line)
                    else:
                        for line in before[i:end_i]: print('-' + line)
                        for line in after[j:end_j]: print('+' + line)
        print('*** End Patch')
    else:
        data = (json.dumps(dict(schema='hc074-remaining-readers-v1', baseline=BASE,
                               rows=[row for row, _, _ in planned]), indent=2) + '\n').encode()
        destination = Path(__file__).with_name('rules.json')
        if args.emit:
            assert not args.write
            print(data.decode(), end='')
            raise SystemExit(0)
        if args.write:
            with destination.open('xb') as output: output.write(data)
        else:
            assert destination.read_bytes() == data, 'fixed caller description differs'
        print(hashlib.sha256(data).hexdigest())
