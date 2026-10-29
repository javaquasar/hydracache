"""Apply only the explicitly approved, preimage-bound local root candidate."""
from pathlib import Path
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import traceback

ROOT = Path(__file__).resolve().parents[2]
BASE = ROOT / 'target/dbus-root-package-20261028'
PACKET = ROOT / 'docs/testing/performance/0.74/local-runs/dbus-root-integration-20261028'
INDEX = '149698f228bba92ece4c5454db982977093899bfc8da6cf5e50c9405c2708ca9'
PKG = 'tools/long-run-supervisor-074'
VENDOR = 'third_party/hc074-dbus/'
sha = lambda raw: hashlib.sha256(raw).hexdigest()


def need(ok, why):
    if not ok:
        raise ValueError(why)


def write(folder, name, value):
    with (folder / name).open('x', encoding='utf8', newline='\n') as out:
        out.write(json.dumps(value, indent=2, sort_keys=True)+'\n')


def member(name):
    raw = (PACKET / 'index.json').read_bytes()
    need(sha(raw) == INDEX, 'whole approved candidate packet')
    row = json.loads(raw)['files'][name]
    raw = (PACKET / 'objects' / row['sha256']).read_bytes()
    need(sha(raw) == row['sha256'] and len(raw) == row['bytes'], 'exact candidate bytes')
    return raw


def destination(root, name):
    need(isinstance(name, str) and name and '\\' not in name and ':' not in name
         and not name.startswith('/') and all(p not in ('', '.', '..') for p in name.split('/')),
         'exact relative source path')
    allowed = (name in ('Cargo.toml', 'Cargo.lock', PKG + '/Cargo.toml')
               or name.startswith(PKG + '/src/') and name.endswith('.rs')
               or any(name.startswith(VENDOR + crate + '/') for crate in ('zbus', 'zbus_macros', 'zbus_utils')))
    need(allowed, 'only approved root candidate paths')
    path = root / name
    need(path.resolve().is_relative_to(root.resolve()), 'source stays in intended root')
    for ancestor in [path, *path.parents]:
        if ancestor == root.parent:
            break
        need(not ancestor.is_symlink() and not ancestor.is_junction(), 'no aliased source/ancestor')
    return path


def verify(root, operations, get):
    need(len({row['path'] for row in operations}) == len(operations), 'unique source operations')
    for row in operations:
        path = destination(root, row['path'])
        raw = get('candidate/after/' + row['path'])
        need(sha(raw) == row['after_sha256'] and len(raw) == row['bytes'], 'exact after payload')
        before = row['before_sha256']
        if before is None:
            need(not path.exists(), 'preserve existing untracked data')
        else:
            need(path.is_file() and sha(path.read_bytes()) == before, 'whole original source preimage')
            need(sha(get('candidate/before/' + row['path'])) == before, 'held original preimage')


def main():
    out = BASE / 'apply'
    out.mkdir(parents=True, exist_ok=False)
    (out / 'producer.py').write_bytes(Path(__file__).read_bytes())
    done = []
    with (out / 'stdout.log').open('xb') as stdout, (out / 'stderr.log').open('xb') as stderr:
        try:
            need(subprocess.check_output(['git', 'branch', '--show-current'], cwd=ROOT).decode().strip()
                 == 'feat/0.74-resp-native-throughput', 'exact authorized 0.74 worktree branch')
            need(not subprocess.check_output(['git', 'status', '--porcelain', '--untracked-files=no'], cwd=ROOT).strip(),
                 'preserve existing tracked user changes')
            recipe = json.loads(member('candidate/fileset.json'))
            need(recipe['schema'] == 'hc074-unapplied-root-dbus-changes-v1'
                 and not recipe['applied'] and len(recipe['operations']) == 291, 'exact prepared local candidate')
            operations = recipe['operations']
            verify(ROOT, operations, member)
            protected = {name: sha((ROOT / name).read_bytes()) for name in
                ('deny.toml', 'docs/testing/performance/0.74/qualification-manifest.toml')}
            write(out, 'intent.json', dict(packet_sha256=INDEX,
                head=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT).decode().strip(),
                operations=operations, protected=protected, local_application_authorized=True,
                source='explicit user request to apply prepared root candidate',
                compiler_gate_complete=False, release_admitted=False, server=False))
            with (out / 'progress.jsonl').open('x', encoding='utf8', newline='\n') as progress:
                for row in operations:
                    verify(ROOT, [row], member)
                    path = destination(ROOT, row['path'])
                    path.parent.mkdir(parents=True, exist_ok=True)
                    temporary = path.with_name(path.name + '.hc074-new')
                    need(not temporary.exists(), 'no foreign temporary source')
                    with temporary.open('xb') as stream:
                        stream.write(member('candidate/after/' + row['path']))
                    os.replace(temporary, path)
                    done.append(row)
                    progress.write(json.dumps(dict(path=row['path'], after_sha256=row['after_sha256']))+'\n')
                    progress.flush()
            for row in operations:
                need(sha(destination(ROOT, row['path']).read_bytes()) == row['after_sha256'], 'applied source postflight')
            need(all(sha((ROOT / name).read_bytes()) == digest for name,digest in protected.items()),
                 'deny/qualification unchanged')
            write(out, 'result.json', dict(applied=True, operations=291, files={r['path']:r['after_sha256'] for r in operations},
                protected=protected, compiler_gate_complete=False, release_admitted=False, server=False,
                original_windows_seconds=[300,60,10]))
        except BaseException as error:
            # Do not reset a partially changed tree or overwrite concurrent work.
            stderr.write(traceback.format_exc().encode())
            write(out, 'refusal.json', dict(type=type(error).__name__, reason=str(error),
                applied_paths=[row['path'] for row in done], automatic_replay=False))
            raise
    print(json.dumps(dict(applied=True, operations=291, release_admitted=False)))


if __name__ == '__main__':
    need(sys.argv[1:] == ['apply'], 'fixed exclusive local application')
    main()
