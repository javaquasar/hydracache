"""Check the exact fixed Tokio source delta; never learn expected pins from HEAD."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
SCOPE = 'tools/long-run-supervisor-074/src'
PREVIOUS = '958b3135a15c00b9f6aad41522cef9d1f6608d3a'
PRODUCER = '2a4a771d48ed6ad54288fd15ea7c75b56f4bd80d'
CHANGED = SCOPE + '/diagnostic_manager_channel.rs'
ADDED = SCOPE + '/diagnostic_manager_alarm_tests.rs'
SHA256 = '1f41899edf172c4ae0a29f17a5fd8a1cee0fd8bf1047c5da5c4ea94b1dca962b'


def git(*args):
    return subprocess.check_output(['git', '--no-replace-objects', *args], cwd=ROOT, timeout=10)


def tree(revision):
    rows = {}
    for line in git('ls-tree', '-r', revision, '--', SCOPE).decode().splitlines():
        meta, name = line.split('\t')
        mode, kind, blob = meta.split()
        assert mode == '100644' and kind == 'blob' and name.startswith(SCOPE + '/')
        assert name not in rows
        rows[name] = dict(mode=mode, blob=blob, canonical_sha256=None)
    return rows


def description():
    previous, current = tree(PREVIOUS), tree(PRODUCER)
    assert (len(previous), len(current)) == (544, 545)
    assert previous.keys() <= current.keys()
    assert sorted(current.keys() - previous.keys()) == [ADDED]
    assert [name for name in previous if previous[name] != current[name]] == [CHANGED]

    def entry(revision, entries, path):
        row = entries[path].copy()
        raw = git('show', f'{revision}:{path}').replace(b'\r\n', b'\n')
        row['canonical_sha256'] = hashlib.sha256(raw).hexdigest()
        return row

    return dict(schema='hc074-inventory545-phase-v1', previous_revision=PREVIOUS,
                revision=PRODUCER, previous_count=544, current_count=545, removed=[],
                changed={CHANGED: dict(previous=entry(PREVIOUS, previous, CHANGED),
                                      current=entry(PRODUCER, current, CHANGED))},
                added={ADDED: entry(PRODUCER, current, ADDED)})


def check():
    raw = (Path(__file__).with_name('rules.json')).read_bytes()
    assert hashlib.sha256(raw).hexdigest() == SHA256
    assert raw == (json.dumps(description(), indent=2) + '\n').encode()
    return dict(descriptor_sha256=SHA256, previous_count=544, current_count=545,
                changed=1, added=1, removed=0, writes=False, admission=False)


if __name__ == '__main__':
    print(json.dumps(check(), sort_keys=True))
