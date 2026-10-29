"""Seal the reviewed first resolved lock for a separate locked all-feature phase."""
from pathlib import Path
import json

import review as r


def prepare():
    original = json.loads((r.BASE / 'freeze/result.json').read_bytes())
    default = json.loads((r.BASE / 'metadata/result.json').read_bytes())
    source = json.loads((r.BASE / 'effects-indexed/source-review.json').read_bytes())
    effects = json.loads((r.BASE / 'effects-indexed/result.json').read_bytes())
    r.need(default['passed'] and effects['lock_delta_reviewed'] and source['upstream_body_review_complete'],
           'actual first metadata and distinct exact source/effect review')
    r.need(r.sha((r.BASE / 'effects-indexed/result.json').read_bytes()) == source['structural_result_sha256'],
           'source conclusions bind exact structural review')
    expected = dict(original['files'])
    expected['workspace/Cargo.lock'] = source['candidate_lock_sha256']
    r.need(r.inventory(r.BASE / 'input') == expected, 'only reviewed first lock changed')
    helpers = dict(original['helpers'])
    for path in ('feature_run.py', 'prepare_features.py', 'test_features.py', 'effects.py', 'test_effects.py'):
        name = 'tools/dbus-root-integration-074/' + path
        helpers[name] = r.sha((r.ROOT / name).read_bytes())
    for path in ('metadata/cargo.stdout.json', 'metadata/result.json', 'metadata/execution.json',
                 'effects-indexed/result.json', 'effects-indexed/source-review.json'):
        name = (r.BASE / path).relative_to(r.ROOT).as_posix()
        helpers[name] = r.sha((r.ROOT / name).read_bytes())
    value = dict(original, files=expected, helpers=helpers, lock_resolved=True,
                 all_features=False, compiler_executed=False, root_adopted=False,
                 original_first_metadata_retained=True)
    r.write(r.BASE, 'resolved-review.json', value)
    print(json.dumps(dict(prepared=True, lock_packages=effects['new_lock_packages'], root_changed=False)))


if __name__ == '__main__':
    prepare()
