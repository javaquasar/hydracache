"""Exact archive-consumer source and inert nested-coordinator render controls."""
from pathlib import Path
import ast
import importlib.util
import json
import re
import unittest

spec = importlib.util.spec_from_file_location('native_archive_consumer_review', Path(__file__).with_name('consumer_prepare.py'))
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


def render(raw):
    raw = raw.replace('/mnt/c/Workspace/prj/jq/cashe/hydracache-074/', p.ROOT.as_posix() + '/')
    tree = ast.parse(raw)
    last = tree.body.pop()
    if not isinstance(last, ast.Expr) or not isinstance(last.value, ast.Call) or last.value.func.id != 'exec':
        raise ValueError('one final coordinator boundary')
    if any(isinstance(n, ast.Call) and isinstance(n.func, ast.Name) and n.func.id == 'exec' for n in ast.walk(tree)):
        raise ValueError('no earlier execution boundary')
    scope = {}
    exec(compile(tree, 'inert-native-archive-driver', 'exec'), scope)
    return scope['text']


class Consumer(unittest.TestCase):
    def test_two_fixed_formatting_projections_preserve_literals_guards_and_fail_stale(self):
        review = json.loads((p.parent.BASE / 'freeze/result.json').read_bytes())
        for name in p.TESTS:
            path = p.parent.BASE / 'input/fd' / p.PKG / name
            raw = path.read_bytes()
            expected = review['files']['fd/' + p.PKG + '/' + name]
            after = p.formatting(name, raw, expected)
            self.assertNotEqual(after, raw)
            self.assertEqual(re.findall(rb'"(?:\\.|[^"\\])*"', after, re.S),
                             re.findall(rb'"(?:\\.|[^"\\])*"', raw, re.S))
            for marker in (b'fn ', b'unsafe', b'assert_eq!', b'writer_shape(', b'::from_secs('):
                self.assertEqual(after.count(marker), raw.count(marker))
            with self.assertRaises(ValueError):
                p.formatting(name, raw + b'\n', expected)

    def test_exact_cohort_renders_to_same_65_cases_and_original_bounds(self):
        raw = Path(__file__).with_name('consumer_unit_native.py').read_text()
        middle = render(raw)
        original = Path(__file__).with_name('fault_unit_native.py').read_text()
        expected = original.replace('dbus-native-20261028/f2', 'dbus-native-source-2-20261028/consumer2', 1).replace(
            '/home/artur/.cache/hydracache-074-dbus-native/20261028-a5',
            '/home/artur/.cache/hydracache-074-dbus-native/20261028-a7', 1)
        self.assertEqual(middle, expected)
        final = render(middle)
        ast.parse(final)
        for value in ("passes_per_compiler=65", "catalog_count=1593",
                      "['--include-ignored', '--exact', *listing['selected']]",
                      "for action in ('list', 'test', 'clippy'):",
                      "[('ok', '65', '0', '0', '0', '1528')]",
                      'time.monotonic() + 1100', 'bound=1200', 'floor=4*1024**3',
                      'poll=5, retirement=5'):
            self.assertIn(value, final)

    def test_prepared_source_is_exact_except_two_test_formatting_and_two_paths(self):
        if not (p.BASE / 'freeze/result.json').exists():
            self.skipTest('consumer freeze not prepared; projection controls run separately')
        actual = json.loads((p.BASE / 'freeze/result.json').read_bytes())
        prior = json.loads((p.parent.BASE / 'freeze/result.json').read_bytes())
        self.assertEqual(p.files(p.BASE / 'input'), actual['files'])
        changed = {n for n in actual['files'] if actual['files'][n] != prior['files'].get(n)}
        self.assertEqual(changed, {'fd/' + p.PKG + '/' + n for n in p.TESTS} |
                         {'baseline/.cargo/config.toml', 'fd/.cargo/config.toml'})
        self.assertTrue(actual['archive_source_exact'] and actual['production_source_equal'])
        self.assertEqual(actual['source_package_recipe_sha256'], p.package.RECIPE_SHA)


if __name__ == '__main__':
    unittest.main()
