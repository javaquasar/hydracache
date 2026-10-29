import ast
import hashlib
import importlib.util
import json
from pathlib import Path
import unittest

HERE=Path(__file__).resolve().parent
spec=importlib.util.spec_from_file_location('native_transport_compile_preparation',HERE/'compile_prepare.py')
p=importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


class Controls(unittest.TestCase):
    def test_source_audit_uses_same_first_lock_without_native_success_promotion(self):
        original=json.loads((p.parent.BASE/'session/native-result.json').read_bytes())
        audit=json.loads((p.parent.BASE/'feature-audit/result.json').read_bytes())
        self.assertFalse(original['passed'])
        self.assertTrue(audit['passed'])
        self.assertFalse(audit['new_cargo_run'])
        raw=(p.parent.BASE/'session/Cargo.lock').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(),audit['resolved_lock_sha256'])

    def test_wrappers_keep_original_bounds_guards_and_exact_unit_coordinator(self):
        for name in ('compile_native.py','compile_run.py','unit_native.py','unit_run.py'):
            raw=(HERE/name).read_text()
            ast.parse(raw)
            self.assertNotIn('1201',raw)
            self.assertNotIn('--ignore-rust-version',raw)
            self.assertNotIn('taskkill',raw)
            self.assertIn('hashlib.sha256(raw)',raw)
        source=p.ROOT/'tools/dbus-legacy-integration-074/unit_native.py'
        self.assertEqual(p.sha(source.read_bytes()),'0b940dcfc7e5ab71a19775e0ece51bf3e7abd990fa0200a1c5c7ecdbf8d4f2b8')


if __name__=='__main__':
    unittest.main()
