"""Local original-owner sandbox observer checks; no service or original delivery."""
import argparse
import importlib.util
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('installed_child_checks', ROOT / 'tools/bootstrap-installed-child-074/check.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
snapshot = previous.snapshot
logger, sha, publish = previous.logger, previous.sha, previous.publish
OUTPUT = ROOT / 'target/bootstrap-sandbox-owner-20261023'
LANES = ('baseline', 'host', '194', '188', 'metadata')
LANES += ('host-final', '194-final', '188-final', 'metadata-final')
FILES = ['crates/xtask/tests/bootstrap_installed_origin_074.rs',
    'docs/testing/performance/0.74/bootstrap-sandbox-owner-plan-20261023.md',
    'tools/bootstrap-sandbox-owner-074/check.py']
FILES += previous.FILES + ['tools/long-run-supervisor-074/src/' + name for name in (
    'diagnostic_bootstrap_sandbox_owner.rs', 'diagnostic_manager_sandbox_query.rs',
    'diagnostic_sandbox_process.rs', 'diagnostic_process.rs', 'diagnostic_manager_channel.rs')]
FILES += ['crates/xtask/tests/support/bootstrap_sandbox_guard.rs',
    'crates/xtask/tests/held_complete_role_census_074.rs', 'crates/xtask/tests/held_material_roles_074.rs',
    'docs/testing/performance/0.74/bootstrap-sandbox-owner-first-result-20261023.json']
FILES += ['tools/bootstrap-sandbox-owner-074/' + name for name in ('retain.py', 'test_controls.py', 'README.md')]
FILES = list(dict.fromkeys(FILES))
def commands(lane):
    if lane not in LANES: raise ValueError('Unknown lane')
    if lane == 'baseline':
        return [('missing', ['cargo', 'test', '--offline', '--locked', '-p', 'xtask', '--test',
            'bootstrap_installed_origin_074', 'bootstrap_sandbox_owner_component_precedes_admission'])]
    kind = lane.split('-', 1)[0]
    steps = previous.commands(kind)
    if kind in ('194', '188'):
        argv = next(argv for name, argv in steps if name == 'installed').copy()
        argv[argv.index('bootstrap_installed_')] = 'bootstrap_sandbox_'
        steps.insert(0, ('sandbox', argv))
    if kind == 'metadata':
        steps[0] = ('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-sandbox-owner-074', '-v'])
    return steps
def run(lane):
    steps = commands(lane)
    if lane == 'baseline' and (ROOT / 'tools/long-run-supervisor-074/src/diagnostic_bootstrap_sandbox_owner.rs').exists():
        raise ValueError('Historical baseline cannot be reexecuted')
    snapshot.OUTPUT, snapshot.FILES, snapshot.commands = OUTPUT, FILES, lambda actual: steps if actual == lane else []
    snapshot.run(lane)
if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=LANES)
    run(parser.parse_args().lane)
