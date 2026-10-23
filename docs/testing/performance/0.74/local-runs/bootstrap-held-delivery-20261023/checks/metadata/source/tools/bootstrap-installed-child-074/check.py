"""Fixed local installed-child consistency controls, never original authority."""
import argparse
import importlib.util
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('installed_material_checks', ROOT / 'tools/bootstrap-installed-material-074/check.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
snapshot = previous.snapshot
logger, sha, publish = previous.logger, previous.sha, previous.publish
OUTPUT = ROOT / 'target/bootstrap-installed-child-20261023'
LANES = ('baseline', 'host', '194', '188', 'metadata', 'host-final', '194-final', '188-final', 'metadata-final')
LANES += ('host-reviewed', '194-reviewed', '188-reviewed')
FILES = ['crates/xtask/tests/bootstrap_installed_origin_074.rs',
    'docs/testing/performance/0.74/bootstrap-installed-child-plan-20261023.md',
    'tools/bootstrap-installed-child-074/check.py']
FILES += previous.FILES + ['tools/long-run-supervisor-074/src/' + name for name in (
    'diagnostic_bootstrap_installed_mode.rs', 'diagnostic_bootstrap_installed_child.rs',
    'diagnostic_bootstrap_installed_child_tests.rs', 'diagnostic_bootstrap_installed_inventory.rs',
    'diagnostic_bootstrap_installed_executable.rs')]
FILES += ['tools/long-run-supervisor-074/tests/support/bootstrap_installed_controls.rs',
    'crates/xtask/tests/support/bootstrap_installed_child_guard.rs',
    'docs/testing/performance/0.74/bootstrap-installed-child-first-result-20261023.json']
FILES += ['tools/bootstrap-installed-child-074/' + name for name in ('retain.py', 'test_controls.py', 'README.md')]
FILES = list(dict.fromkeys(FILES))
def commands(lane):
    if lane not in LANES: raise ValueError('Unknown lane')
    if lane == 'baseline':
        return [('missing', ['cargo','test','--offline','--locked','-p','xtask','--test',
            'bootstrap_installed_origin_074','bootstrap_installed_child_component_precedes_delivery'])]
    kind = lane.split('-', 1)[0]
    steps = previous.commands(kind)
    if kind in ('194', '188'):
        # Private filesystem fixture construction/fork must not overlap another
        # test's executable writer. This is not an original live attempt.
        next(argv for name, argv in steps if name == 'image-cli').append('--test-threads=1')
    if kind == 'metadata':
        steps[0] = ('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-installed-child-074', '-v'])
    return steps
def run(lane):
    steps = commands(lane)
    if lane == 'baseline' and (ROOT / 'tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_child.rs').exists():
        raise ValueError('Historical baseline cannot be reexecuted')
    snapshot.OUTPUT, snapshot.FILES, snapshot.commands = OUTPUT, FILES, lambda actual: steps if actual == lane else []
    snapshot.run(lane)
if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=LANES)
    run(parser.parse_args().lane)
