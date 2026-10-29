"""Record focused offline checks; never imports/boots WSL or launches a unit."""
import ast
import argparse
from pathlib import Path
import subprocess
import sys
import prepare as p

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--final', action='store_true')
args = parser.parse_args()
output = p.OUTPUT / ('final-checks' if args.final else 'checks')
output.mkdir()
for source in Path(__file__).parent.glob('*.py'): ast.parse(source.read_bytes(), filename=str(source))
commands = [('offline', [sys.executable, '-B', '-m', 'unittest', 'discover', '-s', 'tools/held-wsl-native-074', '-v']),
    ('diff', ['git', 'diff', '--check']),
    ('format', ['cargo', 'fmt', '-p', 'hydracache-long-run-supervisor-074', '--', '--check'])] + [(name, ['cargo', 'run', '-p', 'xtask', '--locked', '--', *args])
    for name, args in (('docs', ['doc-check']), ('registry', ['gated-test-check']),
                      ('contract', ['performance-contract-check', '--release', '0.74']))]
for name, argv in commands:
    child = subprocess.run(argv, cwd=p.ROOT, capture_output=True, timeout=300)
    for suffix, raw in (('stdout', child.stdout), ('stderr', child.stderr)):
        with (output / (name + '.' + suffix + '.log')).open('xb') as stream: stream.write(raw)
    p.publish(output / (name + '.json'), dict(argv=argv, exit=child.returncode))
    print(name, child.returncode, flush=True)
    p.need(child.returncode == 0, name + ' failed')
