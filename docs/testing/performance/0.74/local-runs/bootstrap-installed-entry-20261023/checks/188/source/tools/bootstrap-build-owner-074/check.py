"""Fixed static/retained-file/owner-model checks; no original attempt or execution."""
import argparse
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('component_logger', ROOT / 'tools/held-complete-census-074/check.py')
logger = importlib.util.module_from_spec(spec)
spec.loader.exec_module(logger)
OUTPUT = ROOT / 'target/bootstrap-build-owner-20261023'
PACKAGE = 'hydracache-long-run-supervisor-074'
FILES = ['tools/long-run-supervisor-074/src/' + name for name in (
    'diagnostic_builder.rs', 'diagnostic_bootstrap_build.rs', 'diagnostic_bootstrap_build_tests.rs',
    'diagnostic_bootstrap_sealed.rs', 'diagnostic_bootstrap_enrollment.rs',
    'diagnostic_bootstrap_selected_build.rs', 'diagnostic_bootstrap_selected_build_tests.rs',
    'diagnostic_bootstrap_owner.rs', 'diagnostic_bootstrap_owner_tests.rs',
    'diagnostic_held_unit_material.rs', 'diagnostic_consuming_held_join.rs', 'diagnostic_held_unit_projection.rs',
    'diagnostic_held_late_trust.rs', 'diagnostic_held_runtime_trust.rs', 'diagnostic_held_launcher_kernel.rs',
    'diagnostic_original_manager_message.rs', 'diagnostic_root_guardian_raw.rs')]
FILES += ['crates/xtask/tests/' + name for name in ('bootstrap_build_owner_074.rs', 'support/bootstrap_build_guard.rs',
          'bootstrap_enrollment_074.rs', 'held_activation_ingress_074.rs', 'held_bind_alias_refusal_074.rs', 'held_complete_role_census_074.rs', 'held_material_roles_074.rs')]
FILES += ['Cargo.lock', 'docs/COMPAT.md', 'docs/testing/gated-test-registry.toml',
          'docs/testing/performance/0.74/qualification-manifest.toml', 'tools/held-complete-census-074/check.py']
sha, publish = logger.sha, logger.publish


def commands(lane):
    if lane.endswith('-v2'): lane = lane[:-3]
    if lane.endswith('-final'): lane = lane[:-6]
    if lane == 'host':
        targets = ('bootstrap_build_owner_074', 'bootstrap_enrollment_074', 'held_bind_alias_refusal_074',
                   'held_activation_ingress_074', 'held_complete_role_census_074', 'held_material_roles_074', 'held_material_074', 'held_unit_projection_074')
        return [('canaries', ['cargo', 'test', '--offline', '--locked', '-p', 'xtask'] + [a for t in targets for a in ('--test', t)]),
                ('build', ['cargo', 'test', '--offline', '--locked', '-p', PACKAGE, '--lib', 'diagnostic_builder::bootstrap_build::tests', '--', '--nocapture']),
                ('check', ['cargo', 'check', '--offline', '--locked', '-p', PACKAGE, '-p', 'xtask', '--all-targets']),
                ('clippy', ['cargo', 'clippy', '--offline', '--locked', '-p', PACKAGE, '-p', 'xtask', '--all-targets', '--all-features', '--', '-D', 'warnings']),
                ('format', ['cargo', 'fmt', '-p', PACKAGE, '-p', 'xtask', '--', '--check'])]
    if lane == 'metadata':
        return [('controls', ['python', '-B', '-m', 'unittest', 'discover', '-s', 'tools/bootstrap-build-owner-074', '-v']),
                ('docs', ['cargo', 'xtask', 'doc-check']), ('registry', ['cargo', 'xtask', 'gated-test-check']),
                ('contract', ['cargo', 'xtask', 'performance-contract-check', '--release', '0.74']), ('diff', ['git', 'diff', '--check'])]
    if lane not in ('194', '188'): raise ValueError('Unknown lane')
    base = ['wsl.exe', '-d', 'Ubuntu', '-u', 'artur', '--', 'env', 'RUSTUP_AUTO_INSTALL=0',
            'CARGO_TARGET_DIR=/home/artur/.cache/hydracache-074-' + lane + '-check', '/home/artur/.cargo/bin/cargo',
            '+' + ('1.94.0' if lane == '194' else '1.88.0')]
    patterns = [('build', 'diagnostic_builder::bootstrap_build::tests'), ('selection', 'bootstrap_selected_build_'),
                ('owner', 'bootstrap_owner_model_'), ('enrollment', 'bootstrap_enrollment_'),
                ('old-source', 'held_trust_source_retains_independent_selection_and_revalidates'),
                ('old-replacement', 'same_bytes_leaf_replacement_is_sticky_even_after_original_restore'),
                ('old-keys', 'held_key_is_nonweak_and_distinct_from_all_three_old_keys')]
    return [(name, base + ['test', '--offline', '--locked', '--release', '-p', PACKAGE, '--all-features', '--lib', pattern,
                           '--', '--nocapture', '--test-threads=1']) for name, pattern in patterns] + [
            ('check', base + ['check', '--offline', '--locked', '-p', PACKAGE, '--all-targets']),
            ('clippy', base + ['clippy', '--offline', '--locked', '-p', PACKAGE, '--all-targets', '--all-features', '--', '-D', 'warnings'])]


def run(lane):
    logger.OUTPUT, logger.FILES, logger.commands = OUTPUT, FILES, commands
    OUTPUT.mkdir(exist_ok=True)
    logger.run(lane)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('lane', choices=('host', '194', '188', 'metadata', 'host-v2', '194-v2', '188-v2', 'host-final', '194-final', '188-final'))
    run(parser.parse_args().lane)
