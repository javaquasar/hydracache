"""Static boundary audit and existing pure models; no original child or service."""
import argparse
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('boundary_snapshot_runner', ROOT / 'tools/bootstrap-current-image-074/check.py')
runner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(runner)
logger, sha, publish = runner.logger, runner.sha, runner.publish
OUTPUT = ROOT / 'target/bootstrap-child-boundary-20261023'
LANES = ('host', '194', '188', 'metadata', 'host-final', '194-final', '188-final', 'metadata-final')
FILES = ['tools/long-run-supervisor-074/src/' + n for n in (
    'diagnostic_unit.rs', 'diagnostic_held_unit_projection.rs', 'diagnostic_held_unit_projection_tests.rs',
    'diagnostic_mount_namespace.rs', 'diagnostic_worker_policy.rs', 'diagnostic_worker_trust_input.rs',
    'diagnostic_held_runtime_trust.rs', 'diagnostic_bootstrap_sealed.rs', 'diagnostic_bootstrap_current_image.rs',
    'diagnostic_bootstrap_image_entry.rs', 'diagnostic_bootstrap_owner.rs', 'diagnostic_bootstrap_entry_binding.rs')]
FILES += ['Cargo.lock', 'docs/COMPAT.md', 'docs/testing/gated-test-registry.toml',
          'docs/testing/performance/0.74/qualification-manifest.toml', 'crates/xtask/tests/bootstrap_child_boundary_074.rs',
          'tools/bootstrap-current-image-074/check.py', 'tools/bootstrap-child-boundary-074/check.py',
          'tools/bootstrap-child-boundary-074/retain.py', 'tools/bootstrap-child-boundary-074/test_controls.py']


def commands(lane):
    if lane not in LANES: raise ValueError('Unknown lane')
    lane = lane.removesuffix('-final')
    if lane == 'host':
        return [('audit', ['cargo', 'test', '--offline', '--locked', '-p', 'xtask', '--test', 'bootstrap_child_boundary_074', '--', '--nocapture']),
                ('check', ['cargo', 'check', '--offline', '--locked', '-p', 'xtask', '--all-targets']),
                ('clippy', ['cargo', 'clippy', '--offline', '--locked', '-p', 'xtask', '--all-targets', '--all-features', '--', '-D', 'warnings'])]
    if lane == 'metadata':
        return [('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-child-boundary-074', '-v']),
                ('docs', ['cargo', 'xtask', 'doc-check']), ('registry', ['cargo', 'xtask', 'gated-test-check']),
                ('contract', ['cargo', 'xtask', 'performance-contract-check', '--release', '0.74']),
                ('format', ['cargo', 'fmt', '-p', 'xtask', '--', '--check']), ('diff', ['git', 'diff', '--check'])]
    if lane not in ('194', '188'): raise ValueError('Unknown lane')
    base = ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--', 'env', 'RUSTUP_AUTO_INSTALL=0',
            'CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-' + lane + '-check',
            '/home/artur/.cargo/bin/cargo', '+' + ('1.94.0' if lane == '194' else '1.88.0'),
            'test', '--offline', '--locked', '--release', '-p', 'hydracache-long-run-supervisor-074', '--all-features', '--lib']
    return [(name, base + [pattern, '--', '--nocapture', '--test-threads=1']) for name, pattern in (
        ('projection', 'held_unit_projection_'), ('build', 'diagnostic_builder::bootstrap_build::tests'))]


def run(lane):
    runner.OUTPUT, runner.FILES, runner.commands = OUTPUT, FILES, commands
    runner.run(lane)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=LANES)
    run(parser.parse_args().lane)
