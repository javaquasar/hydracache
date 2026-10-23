"""Preserve exact initial source before correcting duplicate test-helper loading."""
import json
import shutil
import check as c

if __name__ == '__main__':
    snapshots = [json.loads((c.OUTPUT / lane / 'source-before.json').read_bytes()) for lane in ('host', '194', '188', 'metadata')]
    if not all(row == snapshots[0] for row in snapshots): raise ValueError('Initial source mismatch')
    if any(c.sha(c.ROOT / name) != digest for name, digest in snapshots[0].items()): raise ValueError('Source drift')
    dest = c.OUTPUT / 'initial-source'
    dest.mkdir()
    for name in snapshots[0]:
        output = dest / name
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(c.ROOT / name, output)
    print('Exact initial source preserved without rerun')
