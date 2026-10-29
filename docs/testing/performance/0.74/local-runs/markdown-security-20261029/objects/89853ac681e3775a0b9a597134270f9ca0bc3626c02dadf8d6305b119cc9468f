import importlib.util
import json
from pathlib import Path
import unittest

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
spec=importlib.util.spec_from_file_location('exact_private_selector',HERE/'private_native.py')
p=importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


class PrivateSelector(unittest.TestCase):
    def test_exact_ignored_case_from_complete_catalog_is_private_only(self):
        old=ROOT/'target/dbus-legacy-integration-20261028/tests/units/fd-194-list/result.json'
        catalog=json.loads(old.read_bytes())['inventory']
        selected=p.choose(catalog)
        self.assertTrue(selected.endswith('::'+p.LEAF))
        self.assertNotIn('original_service::',selected)
        for changed in (catalog[:-1],catalog+[selected],[n for n in catalog if n!=selected]):
            with self.assertRaises(ValueError):p.choose(changed)


if __name__=='__main__':
    unittest.main()
