"""Exact reader edits from a fixed clean predecessor, never observed approval."""
from pathlib import Path
import argparse
import difflib
import hashlib
import json
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
BASE = '181c077abdfbae5a8c4dfe67cee6f1c95519d529'
BOOTSTRAP = [
    ('bootstrap_build_owner_074', 1), ('bootstrap_child_boundary_074', 2),
    ('bootstrap_child_census_074', 3), ('bootstrap_child_reader_074', 3),
    ('bootstrap_current_image_074', 1), ('bootstrap_enrollment_074', 1),
    ('bootstrap_entry_binding_074', 3), ('bootstrap_image_census_074', 2),
    ('bootstrap_installed_origin_074', 1)]
OWNED = 'crates/xtask/tests/owned_case_074.rs'
AUTHORITY = 'crates/xtask/tests/authority_reader_phase_074.rs'
sha = lambda text: hashlib.sha256(text.encode()).hexdigest()


def frozen(path):
    return subprocess.check_output(['git', '--no-replace-objects', 'show', BASE + ':' + path],
                                   cwd=ROOT).decode().replace('\r\n', '\n')


def row(path, edits, append):
    old = frozen(path)
    new = old
    for edit in edits:
        assert new.count(edit['before']) == edit['count'] and edit['count'] > 0
        new = new.replace(edit['before'], edit['after'])
    new += append
    return dict(path=path, before_sha256=sha(old), after_sha256=sha(new), edits=edits, append=append), old, new


def rows():
    result = []
    for name, count in BOOTSTRAP:
        module = 'installed_io.rs' if name == 'bootstrap_installed_origin_074' else 'io.rs'
        result.append(row('crates/xtask/tests/' + name + '.rs', [dict(
            before='std::fs::read_to_string', after='bootstrap_docs::read_to_string', count=count)],
            '\n#[path = "../../../tools/bootstrap-doc-reader-074/' + module + '"]\nmod bootstrap_docs;\n'))
    anchor = '    let expected = format!("{imports}{}", old.replacen(before, after, 1));'
    result.append(row(OWNED, [dict(before=anchor, after=anchor +
        '\n    let expected = bootstrap_phase::installed(&expected);', count=1)],
        '\n#[path = "../../../tools/bootstrap-doc-reader-074/proof.rs"]\nmod bootstrap_phase;\n'))
    before = 'fn raw(path: &str) -> String {\n    std::fs::read_to_string(runtime::root().join(path))\n        .unwrap()\n        .replace("\\r\\n", "\\n")\n}'
    after = 'fn raw(path: &str) -> String {\n    let raw = std::fs::read_to_string(runtime::root().join(path))\n        .unwrap()\n        .replace("\\r\\n", "\\n");\n    bootstrap_phase_reader::read(path, raw)\n}'
    result.append(row(AUTHORITY, [dict(before=before, after=after, count=1)],
        '\n#[path = "../../../tools/bootstrap-doc-reader-074/reader.rs"]\nmod bootstrap_phase_reader;\n'))
    return result


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--write', action='store_true')
    parser.add_argument('--patch', action='store_true')
    parser.add_argument('--emit', action='store_true')
    args = parser.parse_args()
    planned = rows()
    data = (json.dumps(dict(schema='hc074-bootstrap-doc-reader-v1', baseline=BASE,
                           rows=[value[0] for value in planned]), indent=2) + '\n').encode()
    if args.emit:
        assert not args.patch and not args.write
        print(data.decode(), end='')
    elif args.patch:
        assert not args.write
        print('*** Begin Patch')
        for value, old, new in planned:
            path = ROOT / value['path']
            assert path.read_text(encoding='utf-8') == old, 'foreign or partial reader change: ' + str(path)
            print('*** Update File: ' + path.as_posix())
            before, after = old.splitlines(), new.splitlines()
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
        destination = HERE / 'rules.json'
        if args.write:
            with destination.open('xb') as output: output.write(data)
        else:
            assert destination.read_bytes() == data, 'fixed reader description differs'
        print(hashlib.sha256(data).hexdigest())
