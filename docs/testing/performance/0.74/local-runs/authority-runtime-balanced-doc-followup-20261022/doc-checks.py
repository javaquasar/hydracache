"""Capture documentation and local contracts, never performance/live execution."""
import hashlib
import json
from pathlib import Path
import subprocess

root = Path.cwd(); output = root / 'target/authority-runtime-balanced-checks-20261022'
commands = [('format', ['cargo', 'fmt', '-p', 'hydracache-long-run-supervisor-074', '--', '--check'])] + [
    (name, ['cargo', 'run', '-p', 'xtask', '--locked', '--', *args]) for name, args in (
    ('docs', ['doc-check']), ('registry', ['gated-test-check']),
    ('performance', ['performance-contract-check', '--release', '0.74']))]
for name, argv in commands:
    assert not (output / (name + '.json')).exists()
    child = subprocess.run(argv, cwd=root, capture_output=True, timeout=300)
    for suffix, raw in (('stdout', child.stdout), ('stderr', child.stderr)):
        with (output / (name + '.' + suffix + '.log')).open('xb') as stream: stream.write(raw)
    value = dict(argv=argv, exit=child.returncode, stdout_sha256=hashlib.sha256(child.stdout).hexdigest(),
        stderr_sha256=hashlib.sha256(child.stderr).hexdigest())
    with (output / (name + '.json')).open('x') as stream: json.dump(value, stream, sort_keys=True, indent=2)
    print(json.dumps(dict(name=name, **value)), flush=True)
    assert child.returncode == 0, name
