"""Capture local offline checks only; no measured image or native runner invocation."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys

root = Path.cwd()
output = root / 'target/authority-runtime-balanced-checks-20261022'
output.mkdir(exist_ok=True)
commands = [(name, [sys.executable, '-B', path]) for name, path in (
    ('balanced-offline', 'tools/authority-runtime-balanced-074/test_runner.py'),
    ('cpu-offline', 'tools/authority-runtime-cpu-074/test_method.py'),
    ('variance-offline', 'tools/authority-runtime-variance-074/test_analysis.py'),
    ('native-offline', 'tools/authority-runtime-native-074/test_validate.py'),
    ('runtime-review-offline', 'tools/authority-runtime-074/test_review.py'))]
for name, argv in commands:
    assert not (output / (name + '.json')).exists()
    child = subprocess.run(argv, cwd=root, capture_output=True, timeout=240)
    for suffix, raw in (('stdout', child.stdout), ('stderr', child.stderr)):
        with (output / (name + '.' + suffix + '.log')).open('xb') as stream: stream.write(raw)
    row = dict(argv=argv, exit=child.returncode,
        stdout_sha256=hashlib.sha256(child.stdout).hexdigest(), stderr_sha256=hashlib.sha256(child.stderr).hexdigest())
    with (output / (name + '.json')).open('x') as stream: json.dump(row, stream, sort_keys=True, indent=2)
    print(json.dumps(dict(name=name, **row)), flush=True)
    assert child.returncode == 0, name
