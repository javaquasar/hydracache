"""Inert source, ownership and invocation contracts; no context is fabricated."""
import ast
import importlib.util
import json
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[2]
if not (ROOT / 'Cargo.toml').exists():
    ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location('hc074_owner_control_prepare', ROOT / 'tools/dbus-original-owner-074/prepare.py')
r = importlib.util.module_from_spec(spec)
spec.loader.exec_module(r)
read = lambda p: json.loads(p.read_bytes())


class Controls(unittest.TestCase):
    def test_complete_original_source_and_literal_includes(self):
        review = read(r.BASE / 'prepare/result.json')
        self.assertEqual(sum(n.startswith(r.PKG + '/src/') and n.endswith('.rs') for n in review['original_sources']), 545)
        self.assertEqual(len(review['include_sources']), 21)
        for name, digest in (review['original_sources'] | review['include_sources']).items():
            self.assertEqual(r.sha((ROOT / name).read_bytes()), digest)
            if name != 'Cargo.lock':
                self.assertEqual((ROOT / name).read_bytes(), (r.BASE / 'input/baseline' / name).read_bytes())
        # The isolated workspace lock is intentionally resolved. Its integration-
        # test include remains inventoried but those tests are never compiled/run.
        resolution = read(r.BASE / 'resolve-baseline/result.json')
        self.assertEqual(r.sha((r.BASE / 'input/baseline/Cargo.lock').read_bytes()), resolution['files_after']['Cargo.lock'])

    def test_exact_candidate_source_delta(self):
        base = r.BASE / 'input/baseline'
        candidate = r.BASE / 'input/fd'
        before = r.files(base / r.PKG)
        after = r.files(candidate / r.PKG)
        self.assertEqual(set(after) - set(before), {
            'src/diagnostic_manager_channel_v6.rs',
            'src/diagnostic_manager_channel_kernel_v6.rs',
            'src/diagnostic_manager_sandbox_query_v6.rs',
            'src/diagnostic_original_manager_join_v6.rs',
        })
        self.assertEqual({n for n in before if before[n] != after[n]}, {'Cargo.toml', 'src/diagnostic_original_enrollment_frame.rs'})
        frame = 'src/diagnostic_original_enrollment_frame.rs'
        self.assertEqual((candidate / r.PKG / frame).read_bytes(), (base / r.PKG / frame).read_bytes() + b'\n#[path = "diagnostic_manager_channel_v6.rs"]\nmod manager_channel_v6;\n')

    def test_original_receiver_and_kernel_source_exact(self):
        candidate = r.BASE / 'input/fd' / r.PKG / 'src'
        original = ROOT / r.PKG / 'src'
        self.assertEqual((candidate / 'diagnostic_manager_channel_kernel_v6.rs').read_bytes(), (original / 'diagnostic_manager_channel_kernel.rs').read_bytes())
        for name in ('diagnostic_manager_receiver_pin.rs', 'diagnostic_kernel_manager_binding.rs', 'diagnostic_kernel_admission.rs'):
            self.assertEqual((candidate / name).read_bytes(), (original / name).read_bytes())
        receiver = (candidate / 'diagnostic_manager_receiver_pin.rs').read_text(encoding='utf8')
        self.assertIn('OriginalReceiverBinding', receiver)
        self.assertIn('original.require_context', receiver)
        self.assertEqual(receiver.count('channel.inspect().map_err(invalid)'), 2)

    def test_join_preserves_every_original_validation_statement(self):
        original = (r.fixed.SOURCE / 'diagnostic_original_manager_join.rs').read_bytes()
        candidate = (r.BASE / 'input/fd' / r.PKG / 'src/diagnostic_original_manager_join_v6.rs').read_bytes()
        self.assertEqual(candidate, r.join_projection(original))
        with self.assertRaises(ValueError):
            r.join_projection(original + b'\n')
        end = original.index(b'#[cfg(test)]\n#[test]')
        expected = original[:end].replace(b'bind_fixed_manager(', b'bind_fixed_manager_v6_compile_gate(')
        self.assertTrue(candidate.startswith(expected))
        start = original.index(b'fn validation_round(')
        stop = original.index(b"impl<'join", start)
        self.assertEqual(original[start:stop], candidate[start:stop])
        self.assertNotIn(b'mod message_owner;', candidate)
        self.assertNotIn(b'unsafe', candidate[len(expected):])
        self.assertNotIn(b'OriginalReceiverBinding {', candidate)

    def test_canonical_source_packages_and_held_registry_only(self):
        freeze = read(r.BASE / 'freeze/result.json')
        self.assertEqual(r.files(r.BASE / 'input'), freeze['files'])
        allowed = {}
        for path in (ROOT / 'Cargo.lock', ROOT / 'target/dbus-fixed-channel-20261028/api-fixed/input/fd/Cargo.lock'):
            for node in tomllib.loads(path.read_text(encoding='utf8'))['package']:
                if 'checksum' in node:
                    key = (node['name'], node['version'])
                    self.assertIn(allowed.get(key), (None, node['checksum']))
                    allowed[key] = node['checksum']
        for variant in ('baseline', 'fd'):
            nodes = tomllib.loads((r.BASE / 'input' / variant / 'Cargo.lock').read_text(encoding='utf8'))['package']
            self.assertEqual(nodes, freeze['graphs'][variant])
            for node in nodes:
                if 'checksum' in node:
                    self.assertEqual(allowed[(node['name'], node['version'])], node['checksum'])
        _, _, packages = r.fixed.prior.parent.reviewed_packages()
        for crate, files in packages.items():
            self.assertEqual(r.files(r.BASE / 'input/fd/source' / crate), {n: r.sha(v) for n, v in files.items()})

    def test_root_inputs_and_no_admission(self):
        freeze = read(r.BASE / 'freeze/result.json')
        self.assertEqual(r.fixed.prior.parent.parent.fragment.prior.parent.old.root_inputs(), freeze['root_inputs'])
        for name, value in {'actual_original_frame': True, 'actual_receiver_binding': True, 'context_stubs': False, 'library_only': True, 'tests_executed': False, 'system_bus': False, 'root_changed': False, 'release_admitted': False}.items():
            self.assertEqual(freeze[name], value)

    def test_fixed_library_only_owned_commands(self):
        # Load only declarations before native.py's command dispatch, never session.
        path = ROOT / 'tools/dbus-original-owner-074/native.py'
        tree = ast.parse(path.read_text(encoding='utf8'))
        declarations = []
        for node in tree.body:
            if isinstance(node, ast.If) and ast.unparse(node.test).startswith('len(sys.argv) == 4'):
                break
            declarations.append(node)
        namespace = {'__file__': str(path)}
        exec(compile(ast.Module(body=declarations, type_ignores=[]), str(path), 'exec'), namespace)
        namespace['ROOT'] = ROOT
        owner = namespace['configured']()
        for variant in ('baseline', 'fd'):
            for compiler in ('194', '188'):
                for action in ('check', 'clippy'):
                    argv, env, _, _, _, _ = owner.command(f'{variant}-{compiler}-{action}', {})
                    for flag in ('--offline', '--locked', '--lib', '--all-features'):
                        self.assertIn(flag, argv)
                    self.assertNotIn('--all-targets', argv)
                    self.assertIn('hydracache-long-run-supervisor-074', argv)
                    self.assertEqual(env['CARGO_INCREMENTAL'], '0')
                    self.assertIn(('1.94.0' if compiler == '194' else '1.88.0') + '-x86_64-unknown-linux-gnu', argv[0])
                    if action == 'clippy':
                        self.assertEqual(argv[-3:], ['--', '-D', 'warnings'])
        with self.assertRaises(ValueError):
            owner.command('fd-188-test', {})

    def test_lint_correction_only_changes_module_identity_and_anchor(self):
        first = r.BASE / 'input'
        corrected = r.BASE / 'lint-fixed/input'
        review = read(r.BASE / 'lint-fixed/prepare/result.json')
        self.assertEqual(r.files(corrected), review['files'])
        before, after = r.files(first), r.files(corrected)
        receiver = 'fd/' + r.PKG + '/src/diagnostic_manager_receiver_pin_v6.rs'
        self.assertEqual(set(after) - set(before), {receiver})
        core = 'fd/' + r.PKG + '/src/diagnostic_manager_channel_v6.rs'
        kernel = 'fd/' + r.PKG + '/src/diagnostic_manager_channel_kernel_v6.rs'
        self.assertEqual({n for n in before if before[n] != after[n]}, {core, kernel, 'baseline/.cargo/config.toml', 'fd/.cargo/config.toml'})
        self.assertEqual((corrected / receiver).read_bytes(), (r.fixed.SOURCE / 'diagnostic_manager_receiver_pin.rs').read_bytes())
        self.assertEqual((corrected / kernel).read_bytes(), (first / kernel).read_bytes().replace(b'"diagnostic_manager_receiver_pin.rs"', b'"diagnostic_manager_receiver_pin_v6.rs"', 1))
        self.assertEqual((corrected / core).read_text(encoding='utf8'), (first / core).read_text(encoding='utf8').replace('const _: fn(&mut FixedManagerChannel,&str,&str)->std::result::Result<(u32,[u8;16],String),()> = FixedManagerChannel::observe_sandbox;', 'const _: fn(&mut FixedManagerChannel, &str, &str) = |channel, unit, cgroup| { let _ = channel.observe_sandbox(unit, cgroup); };', 1))
        for variant in ('baseline', 'fd'):
            self.assertEqual((corrected / variant / 'Cargo.lock').read_bytes(), (first / variant / 'Cargo.lock').read_bytes())
        self.assertTrue(review['reuse_passing_baseline'])
        self.assertTrue(review['first_failure_retained'])
        self.assertTrue(review['no_lint_exemptions'])

    def test_original_fixed_deadlines_unchanged(self):
        source = ROOT / r.PKG / 'src'
        kernel = (source / 'diagnostic_manager_channel_kernel.rs').read_text(encoding='utf8')
        candidate = (r.BASE / 'input/fd' / r.PKG / 'src/diagnostic_manager_channel_v6.rs').read_text(encoding='utf8')
        self.assertIn('checked_add(2_000_000_000)', kernel)
        self.assertEqual(candidate.count('.method_timeout(Duration::from_millis(250))'), 1)
        for name in ('prepare.py', 'resolve.py', 'freeze.py', 'native.py', 'run.py'):
            ast.parse((ROOT / 'tools/dbus-original-owner-074' / name).read_text(encoding='utf8'))
        native = (ROOT / 'tools/dbus-original-owner-074/native.py').read_text(encoding='utf8')
        self.assertIn('deadline=time.monotonic()+1100', native)
        self.assertIn('floor=4*1024**3,poll=5,retirement=5', native)


if __name__ == '__main__':
    unittest.main()
