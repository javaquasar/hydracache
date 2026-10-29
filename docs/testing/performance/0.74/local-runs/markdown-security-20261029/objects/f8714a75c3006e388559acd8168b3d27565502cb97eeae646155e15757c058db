"""Saved bytes and synthetic fixtures only; never dispatch a WSL client."""
import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock

import debug_transport as d


class DebugTransportTests(unittest.TestCase):
    def test_saved_timeout_and_complete_framing_stay_separate(self):
        result = d.audit(d.load())
        observed = result['observed']
        self.assertEqual(observed['stdout_bytes'], 18864)
        self.assertTrue(observed['standalone_framing_complete'])
        self.assertEqual(observed['client_exit'], 124)
        self.assertTrue(observed['client_timed_out'])
        self.assertFalse(observed['client_exchange_success'])
        self.assertIsNone(observed['guest_exit_code'])
        self.assertIsNone(observed['guest_retirement_confirmed'])
        self.assertFalse(observed['dispatch_allowed'])
        self.assertTrue(all(result[k] is False for k in d.FALSE_FLAGS))

    def test_preflight_refuses_debug_console_and_unknown_fallbacks(self):
        for transport in ('wsl-debug-shell', 'wsl-system', '', 'new-transport'):
            with self.subTest(transport=transport), self.assertRaisesRegex(ValueError, 'dispatch refused'):
                d.require_process_completion(transport)

    def test_markers_exit_zero_eof_or_logout_never_prove_guest_retirement(self):
        for suffix in (b'', b'logout\n', b'\0\x1b[K'):
            raw = b'HC074_ROOT_METADATA_BEGIN\r\nHC074_ROOT_METADATA_END\r\n' + suffix
            for exit_code in (0, 1):
                result = d.assess(raw, exit_code, False)
                self.assertTrue(result['standalone_framing_complete'])
                self.assertEqual(result['client_exchange_success'], exit_code == 0)
                self.assertIsNone(result['guest_retirement_confirmed'])
                self.assertFalse(result['process_completion_contract_supported'])

    def test_echo_missing_duplicate_reversed_and_decorated_markers_are_not_complete(self):
        begin, end = b'HC074_ROOT_METADATA_BEGIN', b'HC074_ROOT_METADATA_END'
        for raw in (b'', begin + b'\n', end + b'\n' + begin + b'\n',
                    begin + b'\n' + end + b'\n' + end + b'\n',
                    b"root# printf '" + begin + b"\\n'\nroot# printf '" + end + b"\\n'\n",
                    b'\0' + begin + b'\n' + end + b'\n'):
            self.assertFalse(d.assess(raw, 124, True)['standalone_framing_complete'])

    def test_invalid_or_relabelled_client_receipt_refuses(self):
        for exit_code, timeout in ((0, True), (1, True), (None, False), (False, False), (124, None)):
            with self.subTest(exit_code=exit_code, timeout=timeout), self.assertRaises(ValueError):
                d.assess(b'', exit_code, timeout)

    def test_widened_timeout_or_original_windows_refuses(self):
        original = d.load()
        for name, changes in (
                ('journal/lookup/debug-shell.json', {'timeout_seconds': 31}),
                ('journal/lookup/debug-shell.json', {'exit': 0}),
                ('journal/subprocess-04-intent.json', {'timeout': 31}),
                ('journal/subprocess-04-intent.json', {'argv': [d.WSL, '--system']}),
                ('journal/subprocess-04-error.json', {'error': 'not a timeout'}),
                ('request/intent.json', {'original_windows': [301, 60, 10]}),
                ('journal/lookup/intent.json', {'original_windows': [300, 61, 10]})):
            files = dict(original)
            files[name] = json.dumps(json.loads(files[name]) | changes).encode()
            with self.subTest(name=name, changes=changes), self.assertRaises(ValueError):
                d.audit(files)

    def test_changed_index_or_object_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            (folder / 'objects').mkdir()
            index = {'schema': 'hc074-root-uac-evidence-v1', 'original_windows': d.WINDOWS,
                     **dict.fromkeys(d.FALSE_FLAGS, False), 'files': {}}
            def save():
                raw = json.dumps(index).encode()
                (folder / 'index.json').write_bytes(raw)
                return d.digest(raw)
            with self.assertRaisesRegex(ValueError, 'Changed input index'):
                d.load(folder, save() + 'changed')
            self.assertEqual(d.load(folder, save()), {})
            index['server'] = True
            with self.assertRaisesRegex(ValueError, 'Different evidence scope'):
                d.load(folder, save())
            index['server'] = False
            index['files'] = {'raw': {'sha256': '../unsafe', 'bytes': 0}}
            with self.assertRaisesRegex(ValueError, 'Unsafe object digest'):
                d.load(folder, save())
            sha = d.digest(b'original')
            (folder / 'objects' / sha).write_bytes(b'changed')
            index['files']['raw'] = {'sha256': sha, 'bytes': 8}
            with self.assertRaisesRegex(ValueError, 'Changed input object'):
                d.load(folder, save())

    def test_changed_original_collector_refuses(self):
        files = d.load()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            tool = root / 'tools/wsl-clock-trace-074'
            tool.mkdir(parents=True)
            (tool / 'root_namespace.py').write_bytes(b'changed')
            with mock.patch.object(d, 'ROOT', root), self.assertRaisesRegex(ValueError, 'Original collector changed'):
                d.audit(files)


if __name__ == '__main__':
    unittest.main()
