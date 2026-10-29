"""Offline controls of the native-only validator; no process is launched here."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import validate as v


class Controls(unittest.TestCase):
    def test_native_validator_has_no_cohort_or_measurement_dispatch(self):
        import ast
        tree = ast.parse(Path(v.__file__).read_text())
        names = {node.func.attr for node in ast.walk(tree) if isinstance(node, ast.Call) and isinstance(node.func, ast.Attribute)}
        self.assertNotIn('run_cohort', names); self.assertNotIn('execute', names)
        self.assertNotIn('wait_method', names); self.assertNotIn('build', names)

    def test_unsafe_attempt_refuses_before_environment(self):
        with patch.object(v.r, 'environment_guard') as guard:
            with self.assertRaises(ValueError): v.run('../escape')
            guard.assert_not_called()

    def test_build_only_never_executes_selected_image(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); output = root / 'out'; output.mkdir()
            paths = {}
            def seed(lane, target, out, role):
                target.mkdir(); image = target / 'image'; image.write_bytes(role.encode()); paths[role] = image
            calls = []
            def invoke(argv, cwd, out, name, env, timeout):
                calls.append(argv)
                if name.endswith('-build'):
                    role = 'baseline' if '-baseline-' in name else 'installed'
                    return json.dumps(dict(reason='compiler-artifact', executable=str(paths[role]), fresh=False,
                        target=dict(name='hydracache_long_run_supervisor_074'), profile=dict(test=True, opt_level='3'))).encode()
                return b''
            with patch.object(v.m.common, 'seed_dependencies', side_effect=seed), patch.object(v.m.common, 'invoke', side_effect=invoke):
                result = v.build_only('194', dict.fromkeys(v.m.ROLE.values(), root), output, root, {}, lambda: 100)
            self.assertEqual(set(result), set(v.m.ROLE.values())); self.assertEqual(len(calls), 7)
            self.assertTrue(all(argv[0] in ('/home/artur/.cargo/bin/rustc', '/home/artur/.cargo/bin/cargo') for argv in calls))
            self.assertTrue(all('--no-run' in argv for argv in calls if 'test' in argv))

    def test_synthetic_script_requires_exact_fixed_arguments(self):
        import cleanup_child as child
        with patch.object(child.sys, 'argv', ['synthetic', 'wrong']), self.assertRaises(SystemExit): child.main()


if __name__ == '__main__': unittest.main(verbosity=2)
