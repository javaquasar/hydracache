"""Inert revision-2 source reconstruction; no embedded code is imported or run."""
from pathlib import Path
import difflib
import importlib.util
import json
import sys

ROOT = Path(__file__).resolve().parents[2]
PATCHES = Path(__file__).parent / 'patches'
RECIPE_SHA = 'c3fe73bd7c470ca3f524c7dc3b84746ceb051fd43a8e5d64737879b66d648a5f'
PRIMARY = 'ab5c211321f2557437f0c93c0bf2e99b82f8dd417a68e95f84590fb8a45c109e'
NATIVE = '5b52a78c032b622f401079ce2b0d895e47ba93504c4cffcf71bcdf024c87f701'
path = ROOT / 'tools/dbus-source-package-074/package.py'
import hashlib
sha = lambda raw: hashlib.sha256(raw).hexdigest()
if sha(path.read_bytes()) != '1b13c005abfbb96b1374e27fcc871ce0a14d87fe955e2097712a9cfb029fadf3':
    raise ValueError('exact inert canonical codec')
spec = importlib.util.spec_from_file_location('native_package_canonical_codec', path)
codec = importlib.util.module_from_spec(spec)
spec.loader.exec_module(codec)
need = codec.require


def member(packet, expected, name):
    folder = ROOT / 'docs/testing/performance/0.74/local-runs' / packet
    raw = (folder / 'index.json').read_bytes()
    need(sha(raw) == expected, 'whole independently pinned predecessor index')
    value = json.loads(raw)
    row = value.get('members', value.get('files'))[name]
    raw = (folder / 'objects' / row['sha256']).read_bytes()
    need(sha(raw) == row['sha256'] and len(raw) == row['bytes'], 'held primary member')
    return raw


def archives(recipe):
    return {crate: member('dbus-msrv-backport-20261028', PRIMARY,
            'capture/primary/' + crate + '-' + row['version'] + '.crate')
            for crate, row in recipe['bases'].items()}


def reviewed():
    raw = (PATCHES / 'recipe.json').read_bytes()
    need(RECIPE_SHA is not None and sha(raw) == RECIPE_SHA, 'whole revision-2 recipe')
    recipe = json.loads(raw)
    objects = {r['after_sha256']: (PATCHES / 'objects' / r['after_sha256']).read_bytes()
               for r in recipe['changes']}
    packages, final = codec.rebuild(recipe, archives(recipe), objects)
    need(codec.rebuild(recipe, archives(recipe), objects) == (packages, final),
         'two exact source-only reconstructions')
    return recipe, packages, final


def export(out):
    recipe, packages, _ = reviewed()
    out.mkdir(parents=True, exist_ok=False)
    for crate, raw in packages.items():
        name = crate + '-' + recipe['bases'][crate]['version'] + '-hc074-source-2.tar'
        (out / name).write_bytes(raw)
    receipt = dict(recipe_sha256=RECIPE_SHA, source_archive_sha256=recipe['source_archive_sha256'],
                   unsigned=True, code_executed=False, root_adopted=False, registry_qualified=False)
    (out / 'receipt.json').write_bytes(codec.encoded(receipt))
    return receipt


def author():
    """Author once from exact already-tested native source; not an acceptance gate."""
    base = ROOT / 'target/dbus-native-20261028/f2'
    freeze_raw = member('dbus-native-transport-20261028', NATIVE, 'f2/freeze/result.json')
    need((base / 'freeze/result.json').read_bytes() == freeze_raw, 'held final source freeze')
    freeze = json.loads(freeze_raw)
    oldraw = (ROOT / 'tools/dbus-source-package-074/patches/recipe.json').read_bytes()
    need(sha(oldraw) == '6ab8b734574c81f54afb4d1ebb88765210101b8afb86d7730b720f1e8eee4576',
         'whole original source recipe')
    old = json.loads(oldraw)
    recipe = dict(old)
    final = {}
    for crate in ('zbus', 'zbus_macros', 'zbus_utils'):
        folder = base / 'input/fd/source' / crate
        held = {}
        for path in folder.rglob('*'):
            need(not path.is_symlink(), 'regular source only')
            if path.is_file():
                name = path.relative_to(folder).as_posix()
                raw = path.read_bytes()
                need(sha(raw) == freeze['files']['fd/source/' + crate + '/' + name],
                     'actual tested source member')
                held[name] = raw
        prior = old['final_members'][crate]
        changed = {n for n in held if sha(held[n]) != prior.get(n)}
        expected = {'Cargo.toml', 'Cargo.toml.orig', 'BACKPORT074.json',
                    'src/connection/socket/mod.rs'} if crate == 'zbus' else set()
        need(set(held) == set(prior) and changed == expected, 'exact native source effects')
        final[crate] = held
    changes, objects, diffs = [], {}, {}
    for crate, raw in archives(old).items():
        prefix = crate + '-' + old['bases'][crate]['version']
        upstream = codec.unpack(raw, prefix)
        need(set(upstream).issubset(final[crate]), 'no source removal')
        for name, body in sorted(final[crate].items()):
            before = upstream.get(name)
            if before == body:
                continue
            digest = sha(body)
            changes.append(dict(crate=crate, path=name, before_sha256=sha(before) if before is not None else None,
                                after_sha256=digest))
            objects[digest] = body
        delta = ''.join(''.join(difflib.unified_diff(
            upstream.get(n, b'').decode().splitlines(True), body.decode().splitlines(True),
            fromfile='upstream/' + n, tofile='native-source-2/' + n))
            for n, body in sorted(final[crate].items()) if upstream.get(n) != body)
        diffs[crate] = delta.encode()
    packages = {c: codec.pack(m, c + '-' + old['bases'][c]['version']) for c, m in final.items()}
    recipe.update(changes=changes, final_members={c: codec.inventory(m) for c, m in final.items()},
        source_archive_sha256={c: sha(v) for c, v in packages.items()},
        review_diff_sha256={c: sha(v) for c, v in diffs.items()},
        source_revision='hc074-zbus-native-fixture-source-2',
        predecessor_index_sha256=NATIVE, tested_consumer_lock_is_separate=True,
        root_adopted=False, published=False, registry_qualified=False, unsigned=True)
    need(codec.rebuild(recipe, archives(recipe), objects) == (packages, final),
         'exact recipe inverse before export')
    PATCHES.mkdir(exist_ok=False)
    (PATCHES / 'objects').mkdir()
    for digest, raw in objects.items():
        (PATCHES / 'objects' / digest).write_bytes(raw)
    for crate, raw in diffs.items():
        (PATCHES / (crate + '.diff')).write_bytes(raw)
    (PATCHES / 'recipe.json').write_bytes(codec.encoded(recipe))
    (PATCHES / '.gitattributes').write_text('objects/* -text -diff\nrecipe.json -text\n*.diff -text\n',
                                         encoding='ascii', newline='\n')
    print(json.dumps(dict(recipe_sha256=sha(codec.encoded(recipe)), changes=len(changes),
                          final_members=sum(map(len, final.values())), code_executed=False)))


if __name__ == '__main__':
    if sys.argv[1:] == ['author']:
        author()
    elif sys.argv[1:] == ['rebuild-only']:
        print(json.dumps(export(ROOT / 'target/dbus-native-source-2-20261028/export')))
    else:
        raise SystemExit('fixed author/rebuild-only mode')
