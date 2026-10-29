import importlib.util
import json
from pathlib import Path
import unittest

spec=importlib.util.spec_from_file_location('native_capacity_preparation',Path(__file__).with_name('capacity_prepare.py'))
p=importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


class Controls(unittest.TestCase):
    def test_old_floor_failure_is_retired_without_a_native_verdict(self):
        original=json.loads((p.parent.BASE/'session/execution.json').read_bytes())
        self.assertEqual(original['classification'],'resource-floor')
        self.assertTrue(original['retired'])
        self.assertEqual(original['exit'],125)
        self.assertFalse((p.parent.BASE/'session/native-result.json').exists())

    def test_prepared_repair_changes_only_two_paths_and_no_code_lock_or_checks(self):
        old=json.loads((p.parent.BASE/'freeze/result.json').read_bytes())['files']
        new=json.loads((p.BASE/'freeze/result.json').read_bytes())['files']
        self.assertEqual(set(old),set(new))
        changed={name for name in old if old[name]!=new[name]}
        self.assertEqual(changed,{'baseline/.cargo/config.toml','fd/.cargo/config.toml'})
        for name in changed:
            before=(p.parent.BASE/'input'/name).read_bytes()
            after=(p.BASE/'input'/name).read_bytes()
            self.assertEqual(after.replace(p.NATIVE.encode(),p.parent.NATIVE.encode()),before)


if __name__=='__main__':
    unittest.main()
