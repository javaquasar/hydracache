"""Fixed local ingress lanes using the existing exclusive component logger."""
import argparse
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('component_logger', ROOT / 'tools/held-complete-census-074/check.py')
logger = importlib.util.module_from_spec(spec)
spec.loader.exec_module(logger)
OUTPUT = ROOT / 'target/held-activation-ingress-20261022'
FILES = ['tools/long-run-supervisor-074/' + name for name in (
    'src/bin/diagnostic_held_launcher.rs', 'src/diagnostic_held_activation_ingress.rs',
    'src/diagnostic_held_launcher_kernel.rs', 'src/diagnostic_held_unit_material.rs',
    'src/diagnostic_held_role_census.rs', 'src/diagnostic_held_unit_projection.rs',
    'src/diagnostic_consuming_held_join.rs', 'src/diagnostic_manager_channel.rs',
    'src/diagnostic_original_manager_message.rs', 'tests/held_launcher.rs',
    'tests/support/held_activation_process.rs')]
FILES += ['crates/xtask/tests/held_activation_ingress_074.rs', 'docs/COMPAT.md',
          'Cargo.lock', 'docs/testing/gated-test-registry.toml',
          'docs/testing/performance/0.74/qualification-manifest.toml',
          'tools/held-complete-census-074/check.py']
PACKAGE = 'hydracache-long-run-supervisor-074'
sha, publish = logger.sha, logger.publish


def commands(lane):
    if lane == 'host':
        tests = ('held_activation_ingress_074', 'held_complete_role_census_074',
                 'held_material_roles_074', 'held_material_074', 'held_unit_projection_074')
        return [('canaries', ['cargo', 'test', '--offline', '--locked', '-p', 'xtask'] +
                 [arg for test in tests for arg in ('--test', test)]),
                ('check', ['cargo', 'check', '--offline', '--locked', '-p', PACKAGE, '-p', 'xtask', '--all-targets']),
                ('clippy', ['cargo', 'clippy', '--offline', '--locked', '-p', PACKAGE, '-p', 'xtask',
                           '--all-targets', '--all-features', '--', '-D', 'warnings']),
                ('format', ['cargo', 'fmt', '-p', PACKAGE, '-p', 'xtask', '--', '--check'])]
    if lane == 'metadata':
        return [('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/held-activation-ingress-074', '-v']),
                ('docs', ['cargo', 'xtask', 'doc-check']),
                ('registry', ['cargo', 'xtask', 'gated-test-check']),
                ('contract', ['cargo', 'xtask', 'performance-contract-check', '--release', '0.74']),
                ('diff', ['git', 'diff', '--check'])]
    if lane not in ('194', '188'): raise ValueError('Unknown lane')
    base = ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--', 'env', 'RUSTUP_AUTO_INSTALL=0',
            'CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-' + lane + '-check',
            '/home/artur/.cargo/bin/cargo', '+' + ('1.94.0' if lane == '194' else '1.88.0')]
    return [('binary', base + ['test', '--offline', '--locked', '--release', '-p', PACKAGE,
                              '--bin', 'diagnostic_held_launcher', '--', '--nocapture', '--test-threads=1']),
            ('process', base + ['test', '--offline', '--locked', '--release', '-p', PACKAGE,
                               '--test', 'held_launcher', 'held_activation_environment', '--', '--nocapture', '--test-threads=1']),
            ('check', base + ['check', '--offline', '--locked', '-p', PACKAGE, '--all-targets']),
            ('clippy', base + ['clippy', '--offline', '--locked', '-p', PACKAGE, '--all-targets', '--all-features', '--', '-D', 'warnings'])]


def run(lane):
    logger.OUTPUT, logger.FILES, logger.commands = OUTPUT, FILES, commands
    logger.run(lane)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=('host', '194', '188', 'metadata'))
    run(parser.parse_args().lane)
