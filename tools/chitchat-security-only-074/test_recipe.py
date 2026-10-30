import copy
from pathlib import Path
import unittest
import candidate as p
import scope as c

class ExactSource(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        path=Path.home()/'.cargo/registry/cache/index.crates.io-1949cf8c6b5b557f/chitchat-0.11.1.crate'
        cls.raw=path.read_bytes()

    def test_steps_preserve_all_algorithms_and_members(self):
        old,rows=p.variants(self.raw)
        self.assertEqual(len(old),21)
        for name,files in rows.items():
            files=dict(files);p.test_style(files,reverse=True)
            self.assertEqual(files['src/serialize.rs'],old['src/serialize.rs'])
            self.assertEqual(files['src/delta.rs'].replace(b"fn as_ref(&self) -> DeltaOpRef<'_> {",b'fn as_ref(&self) -> DeltaOpRef {'),old['src/delta.rs'])
            self.assertEqual(files['src/lib.rs'].replace(b"fn strip_key_prefix(&self, prefix: &str) -> Option<KeyChangeEvent<'_>> {",b'fn strip_key_prefix(&self, prefix: &str) -> Option<KeyChangeEvent> {'),old['src/lib.rs'])
        self.assertEqual(rows['B']['Cargo.toml'],old['Cargo.toml'])
        self.assertEqual(rows['Z']['src/serialize.rs'],rows['U']['src/serialize.rs'])

    def test_foreign_archive_bytes_refuse(self):
        for raw in (self.raw+b'\n',self.raw[:-1],b'not a source package'):
            with self.assertRaises(ValueError):p.variants(raw)

    def test_missing_and_duplicate_preimages_refuse(self):
        for body in (b'absent',b'needle needle'):
            with self.assertRaises(ValueError):p.replace({'member':body},'member',b'needle',b'replacement')

if __name__=='__main__':unittest.main()
