"""Offline runner controls: no WSL, process execution, clock or services."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location('owned_runner', Path(__file__).with_name('check.py'))
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class RunnerControls(unittest.TestCase):
    target = '/home/artur/.cache/hydracache-074-194-check'
    name = 'bootstrap_current_image'

    def artifact(self, executable=None, **changes):
        value = dict(reason='compiler-artifact', target=dict(name=self.name),
                     executable=executable or self.target + '/release/deps/' + self.name + '-123abc')
        value.update(changes)
        return json.dumps(value)

    def test_one_exact_image_ignores_dependencies_without_executable(self):
        rows = [self.artifact(), json.dumps(dict(reason='compiler-artifact', target=dict(name=self.name), executable=None)),
                self.artifact(reason='build-script-executed')]
        self.assertEqual(runner.selected_artifact(rows, self.name, self.target).name,
                         self.name + '-123abc')

    def test_missing_duplicate_wrong_name_and_malformed_refuse(self):
        for rows in ([], [self.artifact(), self.artifact()],
                     [self.artifact(target=dict(name='other'))], ['not json']):
            with self.subTest(rows=rows), self.assertRaises((ValueError, json.JSONDecodeError)):
                runner.selected_artifact(rows, self.name, self.target)

    def test_only_exact_native_cache_and_binary_name(self):
        for path in ('/tmp/' + self.name + '-123abc',
                     self.target + '/release/deps/other-123abc',
                     self.target + '/release/deps/' + self.name + '-123abc.exe',
                     self.target + '/release/deps/' + self.name + '-XYZ',
                     self.target + '/release/deps/../' + self.name + '-123abc'):
            with self.subTest(path=path), self.assertRaises(ValueError):
                runner.selected_artifact([self.artifact(path)], self.name, self.target)

    def test_full_ordinary_and_bootstrap_have_no_selectors(self):
        self.assertEqual(runner.selection('v9-full-ordinary'),
                         (True, 'hydracache_long_run_supervisor_074', []))
        self.assertEqual(runner.selection('v8-bootstrap-isolation'),
                         (False, self.name, []))
        with self.assertRaises(ValueError):
            runner.selection('retry')

    def test_targeted_cases_remain_explicit_and_not_ignored(self):
        self.assertEqual(len(runner.selection('v4-fd')[2]), 2)
        self.assertEqual(runner.selection('v7-frozen-input')[2],
                         ['diagnostic_artifacts::linux::io_binding_tests::'])
        self.assertEqual(runner.selection('v10-transport-eof')[2],
                         ['diagnostic_two_phase_launch::root_enrollment::kernel::transport::'])
        for tag in runner.TAGS:
            self.assertFalse(any(value.startswith('--') for value in runner.selection(tag)[2]))

    def test_snapshot_is_fresh_and_includes_added_and_removed_files(self):
        with tempfile.TemporaryDirectory(prefix='hc074-runner-offline-') as directory:
            root = Path(directory)
            (root / 'Cargo.toml').write_text('a')
            (root / 'Cargo.lock').write_text('b')
            folder = root / 'tools/owned-case-074'
            folder.mkdir(parents=True)
            with patch.object(runner, 'ROOT', root):
                first = runner.snapshot()
                path = folder / 'candidate.rs'
                path.write_text('c')
                second = runner.snapshot()
                self.assertNotEqual(first, second)
                path.write_text('d')
                self.assertNotEqual(second, runner.snapshot())
                path.unlink()
                self.assertEqual(first, runner.snapshot())


if __name__ == '__main__':
    unittest.main()
