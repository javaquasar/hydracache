"""Fixed local codec checks only; no child execution, live owner or admission."""
import argparse
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('entry_assertion_snapshot', ROOT / 'tools/bootstrap-current-image-074/check.py')
snapshot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(snapshot)
previous_spec = importlib.util.spec_from_file_location('installed_origin_checks', ROOT / 'tools/bootstrap-installed-origin-074/check.py')
previous = importlib.util.module_from_spec(previous_spec)
previous_spec.loader.exec_module(previous)
logger, sha, publish = snapshot.logger, snapshot.sha, snapshot.publish
OUTPUT = ROOT / 'target/bootstrap-installed-entry-20261023'
LANES = ('baseline', 'host', '194', '188', 'metadata', 'host-final', '194-final', '188-final', 'metadata-final')
FILES = [
    'tools/long-run-supervisor-074/src/diagnostic_bootstrap_build.rs',
    'tools/long-run-supervisor-074/src/diagnostic_bootstrap_entry_format.rs',
    'crates/xtask/tests/bootstrap_installed_origin_074.rs',
    'crates/xtask/tests/support/bootstrap_installed_guard.rs',
    'docs/testing/performance/0.74/bootstrap-installed-entry-plan-20261023.md',
    'tools/bootstrap-installed-entry-074/check.py',
]
FILES += previous.FILES + [
    'tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_entry_format.rs',
    'tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_entry_format_tests.rs',
    'crates/xtask/tests/support/bootstrap_installed_entry_guard.rs',
    'docs/testing/performance/0.74/bootstrap-installed-entry-first-result-20261023.json',
] + ['tools/bootstrap-installed-entry-074/' + name for name in ('retain.py', 'test_controls.py', 'README.md')]
FILES = list(dict.fromkeys(FILES))


def commands(lane):
    if lane not in LANES: raise ValueError('Unknown lane')
    if lane == 'baseline':
        return [('missing', ['cargo', 'test', '--locked', '--offline', '-p', 'xtask', '--test',
                            'bootstrap_installed_origin_074', 'bootstrap_installed_entry_component_precedes_runtime'])]
    kind = lane.removesuffix('-final')
    steps = previous.commands(kind)
    if kind == 'host':
        steps.insert(0, ('codec', ['cargo', 'test', '--offline', '--locked', '-p',
            'hydracache-long-run-supervisor-074', '--lib', 'bootstrap_installed_entry_', '--', '--nocapture']))
    elif kind == 'metadata':
        steps[0] = ('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-installed-entry-074', '-v'])
    return steps


def run(lane):
    steps = commands(lane)
    if lane == 'baseline' and (ROOT / 'tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_entry_format.rs').exists():
        raise ValueError('Baseline is historical; never reexecute after implementation')
    snapshot.OUTPUT, snapshot.FILES, snapshot.commands = OUTPUT, FILES, lambda actual: steps if actual == lane else []
    snapshot.run(lane)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=LANES)
    run(parser.parse_args().lane)
