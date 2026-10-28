"""Retain root projection/effect proofs and exact unapplied local changeset."""
from pathlib import Path
import json
import subprocess

import review as r
import run as owner

OUT = r.ROOT / 'docs/testing/performance/0.74/local-runs/dbus-root-integration-20261028'


def validate():
    receipt = json.loads((r.BASE / 'resolved-review.json').read_bytes())
    owner.guard(receipt)
    counts = []
    for name, count in (('metadata', 678), ('metadata-all-features', 683)):
        folder = r.BASE / name
        result = json.loads((folder / 'result.json').read_bytes())
        execution = json.loads((folder / 'execution.json').read_bytes())
        r.need(result['passed'] and result['packages'] == count and result['workspace_members'] == 36
               and execution['retired'] and not execution['errors'] and execution['exit'] == 0,
               'complete actual metadata/postflight')
        metadata = json.loads((folder / 'cargo.stdout.json').read_bytes())
        owner.validate_metadata(metadata, receipt)
        r.need(len(metadata['packages']) == count, 'actual graph count, not a compiler/test count')
        counts.append(count)
    review = json.loads((r.BASE / 'effects-indexed/source-review.json').read_bytes())
    r.need(review['upstream_body_review_complete'] and not review['root_adoption_admitted']
           and r.sha((r.BASE / 'effects-indexed/result.json').read_bytes()) == review['structural_result_sha256']
           and r.sha((r.BASE / 'input/workspace/Cargo.lock').read_bytes()) == review['candidate_lock_sha256'],
           'exact separate upstream source review; no root/compiler admission')
    return receipt


def main():
    receipt = validate()
    raw = {}
    for name in ('prepare', 'freeze', 'review-diffs', 'metadata', 'metadata-all-features',
                 'effects', 'effects-indexed', 'first-inert'):
        scope = r.BASE / name
        r.need(scope.is_dir(), 'retain each original context')
        for path in scope.rglob('*'):
            if path.is_file():
                raw[name + '/' + path.relative_to(scope).as_posix()] = path.read_bytes()
        if (scope / 'intent.json').is_file():
            head = json.loads((scope / 'intent.json').read_bytes())['source']['head']
            names = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', head,
                                             'tools/dbus-root-integration-074'], cwd=r.ROOT).decode().splitlines()
            for member in names:
                raw[name + '/committed-producer/' + Path(member).name] = \
                    subprocess.check_output(['git', 'show', head + ':' + member], cwd=r.ROOT)
    raw['resolved-review.json'] = (r.BASE / 'resolved-review.json').read_bytes()
    operations = []
    candidate = r.BASE / 'input/workspace'
    current = r.inventory(candidate)
    originals = receipt['root_files']
    for name, digest in sorted(current.items()):
        if originals.get(name) == digest:
            continue
        allowed = (name in ('Cargo.toml', 'Cargo.lock', r.PKG + '/Cargo.toml')
                   or name.startswith(r.PKG + '/src/') and name.endswith('.rs')
                   or any(name.startswith(r.VENDOR + '/' + c + '/') for c in ('zbus', 'zbus_macros', 'zbus_utils')))
        r.need(allowed and not Path(name).is_absolute() and '..' not in Path(name).parts,
               'exact reviewed root change scope')
        before = originals.get(name)
        path = r.ROOT / name
        if before is None:
            r.need(not path.exists(), 'never overwrite an untracked source file')
        else:
            raw['candidate/before/' + name] = r.checked(name, before)
        body = (candidate / name).read_bytes()
        r.need(r.sha(body) == digest, 'exact candidate payload')
        raw['candidate/after/' + name] = body
        operations.append(dict(path=name, before_sha256=before, after_sha256=digest, bytes=len(body)))
    r.need(len(operations) == 291, 'exact 23 replacements and 268 additions')
    recipe = dict(schema='hc074-unapplied-root-dbus-changes-v1', operations=operations,
        source_parent_index_sha256=r.PARENT, product_root_unchanged=True, applied=False,
        compiler_gate_complete=False, root_adoption_admitted=False, release_admitted=False,
        formatting_gate_pending=True, original_windows_seconds=[300, 60, 10])
    raw['candidate/fileset.json'] = (json.dumps(recipe, indent=2, sort_keys=True)+'\n').encode()
    for path in Path(__file__).parent.iterdir():
        if path.is_file():
            raw['producer/' + path.name] = path.read_bytes()
    for name in ('files.json', 'result.json'):
        raw['resource/cleanup/' + name] = (r.ROOT / 'target/cleanup-old-rlib-review-20261028' / name).read_bytes()
    raw['parent/source-2-index.json'] = (r.ROOT / 'docs/testing/performance/0.74/local-runs/'
                                      'dbus-native-source-2-20261028/index.json').read_bytes()
    raw['source/plan.md'] = (r.ROOT / 'docs/testing/performance/0.74/dbus-root-integration-plan-20261028.md').read_bytes()
    OUT.mkdir(exist_ok=False)
    objects = OUT / 'objects'
    objects.mkdir()
    members = {}
    for name, body in sorted(raw.items()):
        digest = r.sha(body)
        path = objects / digest
        if not path.exists():
            path.write_bytes(body)
        members[name] = dict(sha256=digest, bytes=len(body))
    value = dict(schema_version=1, files=members, root_changed=False, root_adopted=False,
        release_admitted=False, compilation_executed=False, tests_executed=False,
        server=False, services=False, clocks_changed=False, performance_claim=False,
        original_lock_packages=689, candidate_lock_packages=683,
        default_metadata_packages=678, full_feature_metadata_packages=683,
        workspace_members=36, publishable_members=23, source_changes=22,
        local_vendor_members=266, unapplied_operations=291, original_windows_seconds=[300, 60, 10],
        first_inert_and_cache_refusals_retained=True, binaries_excluded=True)
    (OUT / 'index.json').write_text(json.dumps(value, indent=2, sort_keys=True)+'\n', encoding='utf8', newline='\n')
    (OUT / '.gitattributes').write_text('objects/* -text -diff\nindex.json -text\n', encoding='ascii', newline='\n')
    print(json.dumps(dict(members=len(members), objects=len(list(objects.iterdir())),
                         index_sha256=r.sha((OUT / 'index.json').read_bytes()), operations=len(operations))))


if __name__ == '__main__':
    main()
