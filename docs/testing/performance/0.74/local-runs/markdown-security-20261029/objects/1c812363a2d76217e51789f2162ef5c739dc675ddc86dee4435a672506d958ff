import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('legacy_integration_preparation', Path(__file__).with_name('prepare.py'))
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


class Controls(unittest.TestCase):
    def test_each_whole_original_caller_moves_without_test_or_effect_deletion(self):
        for name in p.NAMES:
            original = (p.ROOT / p.PKG / 'src' / name).read_bytes()
            current = p.projection(name, original)
            self.assertTrue(current.startswith(b'//!') or current.startswith(b'use zbus6 as zbus;'))
            self.assertEqual(current.count(b'#[test]'), original.count(b'#[test]'))
            for marker in (b'Connection::system()', b'"StartTransientUnit"', b'"StopUnit"',
                           b'"GetNameOwner"', b'METHOD_TIMEOUT', b'OPERATION_TIMEOUT'):
                self.assertEqual(current.count(marker), original.count(marker), (name, marker))
            self.assertNotIn(b'zbus::blocking', current)
            self.assertNotIn(b'use zbus::wire::OwnedValue;', current)
            self.assertNotIn(b'::{OwnedObjectPath, Value};', current)
            self.assertNotIn(b'        use zbus::wire::Variant as Value;', current)

    def test_stale_whole_body_cannot_be_projected(self):
        for name in p.NAMES:
            raw = (p.ROOT / p.PKG / 'src' / name).read_bytes()
            with self.assertRaises(ValueError):
                p.projection(name, raw + b'\n')

    def test_bridge_is_only_qualified_v2_with_multithread_runtime_preserved(self):
        before = (p.ROOT / 'tools/dbus-runtime-bridge-074/bridge_v2.rs').read_bytes()
        self.assertIn(b'zbus::block_on', before)
        self.assertIn(b'.cache_properties(cache)', before)
        self.assertNotIn(b'tokio::runtime::Builder', before)
        tree = p.PREVIOUS / 'input/fd/tools/long-run-supervisor-074/Cargo.toml'
        self.assertIn('features = ["tokio-multithread", "proxy", "p2p"]', tree.read_text())

    def test_narrow_bridge_keeps_connection_custody_and_drop_in_runtime(self):
        before = (p.ROOT / 'tools/dbus-runtime-bridge-074/bridge_v2.rs').read_bytes()
        current = p.blocking_bridge(before)
        self.assertIn(b'_connection: Connection,\n    inner: Option<zbus::Proxy', current)
        self.assertIn(b'_connection: self.connection,', current)
        self.assertIn(b'Ok(Self {\n            _connection: connection.clone(),\n            inner: Some(proxy),', current)
        self.assertNotIn(b'Ok(Self {\n            connection: connection.clone(),\n            inner: Some(proxy),', current)
        self.assertEqual(before.count(b'connection.clone(),'), current.count(b'connection.clone(),'))
        drop = before.split(b"impl Drop for Proxy<'_> {", 1)[1].split(b'pub struct ProxyBuilder', 1)[0]
        self.assertIn(drop, current)
        for annotation in (b'allow(dead_code)', b'expect(dead_code)'):
            self.assertNotIn(annotation, current)
        with self.assertRaises(ValueError):
            p.blocking_bridge(before + b'\n')


if __name__ == '__main__':
    unittest.main()
