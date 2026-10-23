"""Fixed local codec checks only; no child execution, live owner or admission."""
import argparse
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('entry_assertion_snapshot', ROOT / 'tools/bootstrap-current-image-074/check.py')
snapshot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(snapshot)
OUTPUT = ROOT / 'target/bootstrap-installed-entry-20261023'
LANES = ('baseline', 'host', '194', '188', 'metadata')
FILES = [
    'tools/long-run-supervisor-074/src/diagnostic_bootstrap_build.rs',
    'tools/long-run-supervisor-074/src/diagnostic_bootstrap_entry_format.rs',
    'crates/xtask/tests/bootstrap_installed_origin_074.rs',
    'crates/xtask/tests/support/bootstrap_installed_guard.rs',
    'docs/testing/performance/0.74/bootstrap-installed-entry-plan-20261023.md',
    'tools/bootstrap-installed-entry-074/check.py',
]


def commands(lane):
    if lane == 'baseline':
        return [('missing', ['cargo', 'test', '--locked', '--offline', '-p', 'xtask', '--test',
                            'bootstrap_installed_origin_074', 'bootstrap_installed_entry_component_precedes_runtime'])]
    raise ValueError('Unknown or not yet implemented lane')


def run(lane):
    steps = commands(lane)
    snapshot.OUTPUT, snapshot.FILES, snapshot.commands = OUTPUT, FILES, lambda actual: steps if actual == lane else []
    snapshot.run(lane)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=LANES)
    run(parser.parse_args().lane)
