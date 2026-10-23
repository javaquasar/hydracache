"""Preserve the changed-source overlay for the first source-guard revision."""
import json
import shutil
import check as c

if __name__ == '__main__':
    rows = [json.loads((c.OUTPUT / (lane + '-final') / 'source-before.json').read_bytes()) for lane in ('host', '194', '188', 'metadata')]
    if not all(row == rows[0] for row in rows): raise ValueError('Revision source mismatch')
    if any(c.sha(c.ROOT / name) != digest for name, digest in rows[0].items()): raise ValueError('Source drift')
    dest = c.OUTPUT / 'final-source-overlay'
    dest.mkdir()
    for name, digest in rows[0].items():
        base = c.OUTPUT / 'initial-source' / name
        if not base.is_file() or c.sha(base) != digest:
            output = dest / name
            output.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(c.ROOT / name, output)
    print('First source-guard revision overlay preserved without rerun')
