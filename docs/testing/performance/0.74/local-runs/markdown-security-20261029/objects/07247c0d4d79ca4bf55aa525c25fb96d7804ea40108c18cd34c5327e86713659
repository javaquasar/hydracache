import copy
import importlib.util
from pathlib import Path
import unittest

import lock_review as lock

spec = importlib.util.spec_from_file_location('native_transport_prepare', Path(__file__).with_name('prepare.py'))
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)


def encode(rows):
    import json
    text = 'version = 4\n'
    for row in rows:
        text += '\n[[package]]\n'
        text += ''.join(k + ' = ' + json.dumps(v) + '\n' for k, v in row.items())
    return text.encode()


def fixture():
    root = dict(name='hydracache-long-run-supervisor-074', version='0.0.0',
                dependencies=['zbus 5.19.0', 'zbus 6.1.0', 'serde'])
    old = [root, dict(name='zbus', version='5.19.0', source='registry+fixture', checksum='a'*64,
                     dependencies=['old-only', 'serde']),
           dict(name='zbus', version='6.1.0', dependencies=['serde']),
           dict(name='serde', version='1.0.228', source='registry+fixture', checksum='b'*64),
           dict(name='old-only', version='1.0.0', source='registry+fixture', checksum='c'*64)]
    new = copy.deepcopy([old[0], old[2], old[3]])
    new[0]['dependencies'] = ['zbus', 'serde']
    return old, new


class Controls(unittest.TestCase):
    def test_only_old_graph_and_root_edge_disappear_with_semantic_alias_normalization(self):
        old, new = fixture()
        result = lock.review(encode(old), encode(new))
        self.assertTrue(result['old_zbus_absent'])
        self.assertEqual(result['new_packages'], 3)
        self.assertFalse(result['admission'])

    def test_added_nodes_checksum_edge_duplicate_or_retained_old_backend_refuse(self):
        for case in ('added', 'checksum', 'edge', 'root', 'duplicate', 'old'):
            old, new = fixture()
            if case == 'added': new.append(dict(name='foreign', version='1.0.0'))
            if case == 'checksum': new[-1]['checksum'] = 'd'*64
            if case == 'edge': new[1]['dependencies'] = []
            if case == 'root': new[0]['dependencies'] = ['zbus']
            if case == 'duplicate': new.append(copy.deepcopy(new[1]))
            if case == 'old': new.append(copy.deepcopy(old[1]))
            with self.subTest(case=case), self.assertRaises(ValueError):
                lock.review(encode(old), encode(new))

    def test_transport_keeps_all_existing_cases_bounds_wire_and_atomic_guard(self):
        raw = (p.PREVIOUS / 'input/fd' / p.PKG / 'src/diagnostic_dispatch_negotiated_tests.rs').read_bytes()
        current = p.transport(raw)
        self.assertEqual(raw.count(b'#[test]'), current.count(b'#[test]'))
        self.assertEqual(raw.count(b'#[ignore'), current.count(b'#[ignore'))
        for marker in (b'Duration::from_secs(3)', b'Duration::from_secs(2)',
                       b'atomic_receive::receive', b'peer.uid()', b'peer.gid()', b'peer.pid()',
                       b'wire.writes.push((count, fds.len()))'):
            self.assertEqual(raw.count(marker), current.count(marker), marker)
        self.assertNotIn(b'::zbus::', current)
        self.assertIn(b'self.0.peer_credentials().await', current)
        self.assertIn(b'hc074_unix_fixture(socket)', current)
        with self.assertRaises(ValueError): p.transport(raw + b'\n')

    def test_dto_remains_optional_and_fd_alias_control_on_one_backend(self):
        raw = (p.PREVIOUS / 'input/fd' / p.PKG / 'src/diagnostic_negotiated_credentials_adapter.rs').read_bytes()
        current = p.credentials(raw)
        self.assertEqual(raw.count(b'#[test]'), current.count(b'#[test]'))
        for marker in (b'old.unix_group_ids()', b'old.linux_security_label()', b'try_clone_to_owned()',
                       b'assert_ne!(alias, number)', b'libc::FD_CLOEXEC'):
            self.assertIn(marker, current)
        self.assertNotIn(b'::zbus::', current)
        self.assertNotIn(b'new_dbus', current)
        with self.assertRaises(ValueError): p.credentials(raw + b'\n')

    def test_nondefault_helper_uses_actual_builder_runtime_and_unix_ops(self):
        raw = Path(__file__).with_name('unix_fixture.rs').read_text()
        self.assertIn('feature = "hc074-unix-fixture"', raw)
        self.assertIn('Runtime::default_for_build()?', raw)
        self.assertIn('io::registered(&runtime, stream, crate::runtime::io::UnixOps)?', raw)
        for forbidden in ('setuid', 'getrandom', 'new_multi_thread', 'libc::recvmsg', 'libc::sendmsg'):
            self.assertNotIn(forbidden, raw)


if __name__ == '__main__':
    unittest.main()
