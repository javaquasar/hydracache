"""One fixed read-only documentation command under the existing Windows Job."""
from pathlib import Path
import hashlib
import json
import os
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT/'tools/dbus-root-package-074'))
import common as c

OUT = ROOT/'docs/testing/performance/0.74/local-runs/dbus-root-package-review-20261029'
BINARY = ROOT/'target/debug/xtask.exe'
ARGV = [str(BINARY), 'doc-check', '--root', str(ROOT)]


if sys.argv[1:] == ['worker']:
    _, env = c.windows_environment()
    os.environ.clear()
    os.environ.update(env)
    owner = c.owned()
    value = owner._owned(owner.load_job().WindowsJob(), ARGV, str(ROOT), c.capacity, bound=1200)
    c.write(OUT, 'doc-check-execution.json', value)
    raise SystemExit(value['exit'])

c.need(not sys.argv[1:], 'one fixed doc check')
with (OUT/'doc-check.stdout.log').open('xb') as stdout, (OUT/'doc-check.stderr.log').open('xb') as stderr:
    c.write(OUT, 'doc-check-intent.json', dict(argv=ARGV, binary_sha256=hashlib.sha256(BINARY.read_bytes()).hexdigest(),
        source='unchanged xtask doc-check implementation; existing local binary', server=False, release_admitted=False))
    value = subprocess.run([sys.executable, '-B', str(Path(__file__)), 'worker'], cwd=ROOT,
        stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr, timeout=1210)
execution = json.loads((OUT/'doc-check-execution.json').read_bytes())
c.need(value.returncode == 0 and execution['exit'] == 0 and execution['retired'] and not execution['errors'],
       'fixed documentation check and original Job retirement')
print(json.dumps(dict(doc_check=True, retired=True)))
