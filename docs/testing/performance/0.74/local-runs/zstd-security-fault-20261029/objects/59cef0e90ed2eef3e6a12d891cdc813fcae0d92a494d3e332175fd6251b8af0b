"""Coordinator controls: source-only capture, no package execution."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import syntax_audit_v5 as candidate


class CaptureControls(unittest.TestCase):
    def test_exact_capture_uses_contextual_resolver_then_restores_previous(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            original = candidate.previous.module_edges
            def capture(root, build, folder):
                self.assertIs(candidate.previous.module_edges, candidate.module_context_v5.module_edges)
                self.assertEqual((root, build, folder), (out, out, out))
            with patch.object(candidate.previous, 'capture', side_effect=capture):
                candidate.capture(out, out, out)
            self.assertIs(candidate.previous.module_edges, original)
            self.assertIn('"execution_effects_cleared": false', (out / 'resolver.json').read_text())

    def test_failed_capture_restores_scanner_and_does_not_write_success_marker(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            original = candidate.previous.module_edges
            with patch.object(candidate.previous, 'capture', side_effect=ValueError('first refusal')):
                with self.assertRaisesRegex(ValueError, 'first refusal'):
                    candidate.capture(out, out, out)
            self.assertIs(candidate.previous.module_edges, original)
            self.assertFalse((out / 'resolver.json').exists())

    def test_previous_scanner_bytes_cannot_be_refreshed_from_observations(self):
        with patch.object(Path, 'read_bytes', return_value=b'unreviewed'):
            with patch.object(candidate.previous, 'capture') as capture:
                with self.assertRaisesRegex(ValueError, 'previous syntax scanner changed'):
                    candidate.capture(Path('.'), Path('.'), Path('.'))
                capture.assert_not_called()

    def test_receipt_cannot_overwrite_an_existing_marker(self):
        with tempfile.TemporaryDirectory() as directory:
            out = Path(directory)
            marker = out / 'resolver.json'
            marker.write_bytes(b'first')
            original = candidate.previous.module_edges
            with patch.object(candidate.previous, 'capture'):
                with self.assertRaises(FileExistsError):
                    candidate.capture(out, out, out)
            self.assertEqual(marker.read_bytes(), b'first')
            self.assertIs(candidate.previous.module_edges, original)


if __name__ == '__main__':
    unittest.main()
