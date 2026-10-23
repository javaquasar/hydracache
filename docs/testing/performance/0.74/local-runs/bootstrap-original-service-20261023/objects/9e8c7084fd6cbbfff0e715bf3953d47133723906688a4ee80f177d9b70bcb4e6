import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import check as c

class Controls(unittest.TestCase):
    def test_durable_json_uses_writable_exclusive_handle(self):
        import json
        spec=importlib.util.spec_from_file_location('native_write',c.ROOT/'tools/bootstrap-original-service-074/native_fixture.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as tmp:
            target=Path(tmp)/'intent.json'
            module.durable_json(target,{'no_dispatch':True})
            self.assertEqual(json.loads(target.read_text()),{'no_dispatch':True})
            with self.assertRaises(FileExistsError): module.durable_json(target,{'changed':True})
    def test_checks_cannot_launch_ignored_or_privileged_tests(self):
        for lane in ('host','194','188','metadata'):
            for _, argv in c.commands(lane):
                for forbidden in ('--ignored','ssh','sudo','systemctl','busctl','root','native_fixture.py'):
                    self.assertNotIn(forbidden,argv)
    def test_unknown_lane(self):
        for lane in ('server','native','qualification'):
            with self.assertRaises(ValueError): c.commands(lane)
    def test_first_result_exclusive(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(c.snapshot,'OUTPUT',Path(tmp)), patch.object(c.snapshot,'FILES',[]), patch.object(c.snapshot,'commands',lambda _:[]):
            c.snapshot.run('fixture')
            with self.assertRaises(FileExistsError): c.snapshot.run('fixture')
    def test_shell_has_no_broad_cleanup_or_existing_service_actions(self):
        raw=(c.ROOT/'tools/bootstrap-original-service-074/native_fixture.sh').read_text()
        self.assertNotIn('rm -r',raw)
        self.assertNotIn('systemctl restart',raw)
        self.assertNotIn('systemctl stop hydracache-performance-supervisor',raw)
        self.assertIn('trap cleanup EXIT HUP INT TERM',raw)
        self.assertLess(raw.index('getent $entry'),raw.index('groupadd --gid'))
        self.assertLess(raw.index('pgrep -u 986'),raw.index('userdel hydracache-perf'))
        self.assertIn('cmp -s "/etc/$name" "$temporary/$name.original"',raw)
        self.assertEqual(raw.count('privileged_original_manager_service_once --ignored'),1)
    def test_native_requires_fixed_workspace_and_immutable_first_lane(self):
        spec=importlib.util.spec_from_file_location('native',c.ROOT/'tools/bootstrap-original-service-074/native_fixture.py')
        module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
        with self.assertRaises(ValueError): module.linux(Path('D:/unrelated'))
        source=(c.ROOT/'tools/bootstrap-original-service-074/native_fixture.py').read_text()
        self.assertIn('exist_ok=False',source)
        self.assertNotIn('ssh',source)
        self.assertIn('raise SystemExit(result.returncode)',source)

if __name__=='__main__': unittest.main()
