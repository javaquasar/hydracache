"""Same fixed owned metadata phase, now all features on the reviewed locked graph."""
from pathlib import Path
import json
import sys
import tomllib

import review as r

PARENT = Path(__file__).with_name('run.py')
PARENT_SHA = 'edba500d7f834b89ef329530eb09b02feb234f4acee8951c3d60f96b3318e02d'


def render():
    raw = PARENT.read_bytes()
    r.need(r.sha(raw) == PARENT_SHA, 'same original qualified metadata/source/Job owner')
    text = raw.decode()
    for old, new, count in (
        ("OUT = r.BASE / 'metadata'", "OUT = r.BASE / 'metadata-all-features'", 1),
        ("r.BASE / 'freeze/result.json'", "r.BASE / 'resolved-review.json'", 2),
        ("[str(CARGO), 'metadata', '--offline', '--format-version'",
         "[str(CARGO), 'metadata', '--offline', '--locked', '--all-features', '--format-version'", 1),
        ('locked=False, only_candidate_lock_may_change=True', 'locked=True, only_candidate_lock_may_change=False', 1),
        ('guard(review, lock_may_change=True)', 'guard(review)', 2),
        ('source_unchanged_except_candidate_lock=True', 'source_unchanged=True', 2),
    ):
        r.need(text.count(old) == count, 'exact all-feature boundary: ' + old)
        text = text.replace(old, new)
    old = '            validate_metadata(metadata, review)\n'
    new = old + '''            locked = tomllib.loads((r.BASE / 'input/workspace/Cargo.lock').read_text())['package']
            expected = {(p['name'], p['version'], p.get('source')) for p in locked}
            actual = {(p['name'], p['version'], p['source']) for p in metadata['packages']}
            r.need(len(expected) == len(locked) and actual == expected,
                   'all lock identities present in the full-feature metadata graph')
'''
    r.need(text.count(old) == 1, 'one complete metadata validation site')
    return text.replace(old, new, 1)


if __name__ == '__main__':
    # The parent worker modes remain exact. No user-selectable command or target.
    r.need(sys.argv[1:] in (['metadata'], ['--worker'], ['--owned-worker']), 'fixed all-feature metadata phase')
    exec(compile(render(), str(PARENT), 'exec'), globals())
