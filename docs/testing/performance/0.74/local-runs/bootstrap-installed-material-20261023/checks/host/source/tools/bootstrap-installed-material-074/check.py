"""Local original-owned sealed record controls; no original live delivery."""
import argparse
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('entry_checks', ROOT / 'tools/bootstrap-installed-entry-074/check.py')
previous = importlib.util.module_from_spec(spec)
spec.loader.exec_module(previous)
snapshot = previous.snapshot
logger, sha, publish = previous.logger, previous.sha, previous.publish
OUTPUT = ROOT / 'target/bootstrap-installed-material-20261023'
LANES = ('baseline', 'host', '194', '188', 'metadata', 'host-final', '194-final', '188-final', 'metadata-final')
FILES = ['crates/xtask/tests/bootstrap_installed_origin_074.rs',
    'tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_owner.rs',
    'docs/testing/performance/0.74/bootstrap-installed-material-plan-20261023.md',
    'tools/bootstrap-installed-material-074/check.py']
FILES += previous.FILES + ['tools/long-run-supervisor-074/src/' + n for n in (
    'diagnostic_bootstrap_installed_entry_census.rs', 'diagnostic_bootstrap_installed_entry_census_tests.rs',
    'diagnostic_bootstrap_installed_entry_resource.rs', 'diagnostic_bootstrap_installed_entry_resource_tests.rs',
    'diagnostic_bootstrap_installed_entry_owner.rs', 'diagnostic_bootstrap_installed_entry_owner_tests.rs')]
FILES += ['crates/xtask/tests/support/bootstrap_installed_material_guard.rs']
FILES += ['tools/bootstrap-installed-material-074/' + n for n in ('retain.py', 'test_controls.py', 'README.md')]
FILES = list(dict.fromkeys(FILES))


def commands(lane):
    if lane not in LANES: raise ValueError('Unknown lane')
    if lane == 'baseline':
        return [('missing', ['cargo', 'test', '--offline', '--locked', '-p', 'xtask', '--test',
            'bootstrap_installed_origin_074', 'bootstrap_installed_material_component_precedes_delivery'])]
    kind = lane.removesuffix('-final')
    steps = previous.commands(kind)
    if kind == 'metadata':
        steps[0] = ('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-installed-material-074', '-v'])
    return steps


def run(lane):
    steps = commands(lane)
    if lane == 'baseline' and (ROOT / 'tools/long-run-supervisor-074/src/diagnostic_bootstrap_installed_entry_owner.rs').exists():
        raise ValueError('Historical baseline cannot be reexecuted')
    snapshot.OUTPUT, snapshot.FILES, snapshot.commands = OUTPUT, FILES, lambda actual: steps if actual == lane else []
    snapshot.run(lane)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=LANES)
    run(parser.parse_args().lane)
