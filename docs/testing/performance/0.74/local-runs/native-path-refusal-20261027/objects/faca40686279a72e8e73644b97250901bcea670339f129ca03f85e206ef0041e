"""Local bounded data/refusal controls; no product or server operation."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import contract as c
import native
import prepare


class Contract(unittest.TestCase):
    def sample(self):
        return dict(schema='hc074-native-frontier-transfer-v1', source='a' * 40,
            tree={'owned.txt': dict(mode='100644', blob='b' * 40, bytes=3)},
            refs={'refs/heads/main': 'a' * 40}, bundle_sha256='c' * 64,
            bundle_bytes=123, qualifier_sha256=c.QUAL_SHA)

    def test_exact_request_and_tree_refs(self):
        value = self.sample()
        self.assertEqual(c.request(c.decode(json.dumps(value).encode())), value)
        data = b'100644 blob ' + b'b' * 40 + b' 3\towned.txt\0'
        self.assertEqual(c.tree(data), value['tree'])
        self.assertEqual(c.refs(b'a' * 40 + b' refs/heads/main\n'), value['refs'])
        self.assertEqual(c.blob_digest(b'test\n'), '9daeafb9864cf43055ae93beb0afd6c7d144bfa4')

    def test_unsafe_alias_duplicate_and_nonregular_tree_refuse(self):
        for name in ('', '.', '../outside', '/absolute', './alias', 'a//b', 'a/../b', '.git/config',
                     'a/.GIT/config', 'C:/escape', 'a\\b', 'bad\npath'):
            self.assertFalse(c.safe_path(name), name)
        good = b'100644 blob ' + b'b' * 40 + b' 3\towned.txt\0'
        for bad in (good[:-1], good + good, good.replace(b'100644', b'120000'),
                    good.replace(b'100644', b'160000'), good.replace(b' 3\t', b' -\t'),
                    good.replace(b'owned.txt', b'../escape')):
            with self.assertRaises((ValueError, UnicodeError)):
                c.tree(bad)

    def test_request_does_not_accept_partial_extra_or_changed_identity(self):
        good = self.sample()
        for field, change in [('source', 'short'), ('qualifier_sha256', '0' * 64),
                              ('bundle_sha256', '0' * 63), ('bundle_bytes', -1),
                              ('bundle_bytes', True), ('refs', {}), ('tree', {})]:
            bad = copy.deepcopy(good)
            bad[field] = change
            with self.assertRaises(ValueError): c.request(bad)
        for field in good:
            bad = copy.deepcopy(good)
            del bad[field]
            with self.assertRaises(ValueError): c.request(bad)
        bad = copy.deepcopy(good)
        bad['allow_retry'] = True
        with self.assertRaises(ValueError): c.request(bad)
        for field, change in [('mode', '100755x'), ('blob', 'bad'), ('bytes', -1)]:
            bad = copy.deepcopy(good)
            bad['tree']['owned.txt'][field] = change
            with self.assertRaises(ValueError): c.request(bad)

    def test_ref_duplicates_nonrelative_and_duplicate_json_refuse(self):
        good = b'a' * 40 + b' refs/heads/main\n'
        for bad in (good + good, good.replace(b'refs/heads', b'../heads'),
                    good.replace(b'refs/heads', b'refs/.git'), b'bad refs/heads/main\n'):
            with self.assertRaises(ValueError): c.refs(bad)
        with self.assertRaisesRegex(ValueError, 'duplicate JSON'):
            c.decode(b'{"source":1,"source":2}')

    def test_physical_capacity_includes_full_tree_and_both_bundles(self):
        need = c.FLOOR + c.SLACK + 10 + 2 * 20
        self.assertEqual(c.space(need, 10, 20), need)
        with self.assertRaisesRegex(ValueError, 'physical backing'):
            c.space(need - 1, 10, 20)

    def test_git_preparation_environment_is_not_inherited_execution_authority(self):
        actual = c.git_environment(dict(PATH='/usr/bin', GIT_DIR='foreign', GIT_WORK_TREE='foreign',
            GIT_CONFIG_COUNT='1', GIT_CONFIG_KEY_0='core.sshCommand', GIT_SSH_COMMAND='foreign', GIT_ASKPASS='foreign'))
        self.assertEqual(actual['PATH'], '/usr/bin')
        for name in ('GIT_DIR', 'GIT_WORK_TREE', 'GIT_CONFIG_COUNT', 'GIT_SSH_COMMAND', 'GIT_ASKPASS'):
            self.assertNotIn(name, actual)
        self.assertEqual(actual['GIT_CONFIG_GLOBAL'], '/dev/null')
        self.assertEqual(actual['GIT_CONFIG_NOSYSTEM'], '1')

    def test_native_content_drift_and_nonregular_refuse_without_git_status(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory).resolve()
            file = root / 'owned.txt'
            file.write_bytes(b'abc')
            row = dict(mode='100644', blob=c.blob_digest(b'abc'), bytes=3)
            listing = b'100644 blob ' + row['blob'].encode() + b' 3\towned.txt\0'
            with patch.object(native, 'git', return_value=listing) as git:
                file.write_bytes(b'bad')
                with self.assertRaisesRegex(ValueError, 'bytes differ'):
                    native.validate_files(root, {'owned.txt': row}, lambda: None)
                self.assertEqual(git.call_count, 1)
                file.unlink()
                file.mkdir()
                with self.assertRaisesRegex(ValueError, 'nonregular'):
                    native.validate_files(root, {'owned.txt': row}, lambda: None)

    def test_native_request_binding_refuses_before_directory_creation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            request = root / 'request.json'
            request.write_text(json.dumps(self.sample()))
            base = root / 'never-created'
            with patch.object(native, 'BASE', base), patch.object(native, 'parents'):
                with self.assertRaisesRegex(ValueError, 'request binding'):
                    native.run(request, '0' * 64)
                self.assertFalse(base.exists())

    def test_export_inventory_keeps_full_scope_and_refuses_dirty_or_stale_head(self):
        value = self.sample()
        listing = b'100644 blob ' + b'b' * 40 + b' 3\towned.txt\0'
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def observed(root, *args):
                return {'rev-parse': ('a' * 40 + '\n').encode(), 'status': b'',
                    'ls-tree': listing, 'for-each-ref': b'a' * 40 + b' refs/heads/main\n'}[args[0]]
            with patch.object(prepare, 'git', side_effect=observed) as git, \
                    patch.object(prepare.c, 'sha', return_value=c.QUAL_SHA), \
                    patch.object(Path, 'read_bytes', return_value=b'qualifier'):
                self.assertEqual(prepare.inventory(root, value['source']), (value['tree'], value['refs']))
                self.assertEqual([call.args[1] for call in git.call_args_list], ['rev-parse', 'status', 'ls-tree', 'for-each-ref'])
            with patch.object(prepare, 'git', return_value=b'wrong\n') as git:
                with self.assertRaisesRegex(ValueError, 'exact explicit HEAD'):
                    prepare.inventory(root, value['source'])
                self.assertEqual(git.call_count, 1)
            with patch.object(prepare, 'git', side_effect=[b'a' * 40 + b'\n', b' M dirty']) as git:
                with self.assertRaisesRegex(ValueError, 'commit source'):
                    prepare.inventory(root, value['source'])
                self.assertEqual(git.call_count, 2)


if __name__ == '__main__': unittest.main()
