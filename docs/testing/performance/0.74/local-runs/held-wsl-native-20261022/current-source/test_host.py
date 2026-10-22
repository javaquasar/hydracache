"""Synthetic Windows host orchestration controls, never invoking WSL."""
import copy
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import host as h


def state(added=False):
    value = dict(default='ubuntu-guid', global_config_sha256=None, free_bytes=40 * 1024**3,
        distros={'ubuntu-guid': dict(DistributionName=dict(value='Ubuntu'), Flags=dict(value=15))})
    if added:
        value['distros']['new-guid'] = dict(DistributionName=dict(value=h.p.DISTRO), BasePath=dict(value=str(h.p.INSTALL)))
    return value


class Controls(unittest.TestCase):
    def test_only_exact_new_environment_is_permitted_and_old_context_preserved(self):
        before = state(); after = state(True)
        h.preserved(before, after)
        mutations = [dict(default='new-guid'), dict(global_config_sha256='changed'), dict(distros={}),
            dict(distros={**after['distros'], 'other': after['distros']['new-guid']})]
        for mutation in mutations:
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                h.preserved(before, {**after, **mutation})
        for key, field, value in (('ubuntu-guid', 'Flags', 7), ('new-guid', 'DistributionName', 'Other'),
                ('new-guid', 'BasePath', 'C:/elsewhere')):
            changed = copy.deepcopy(after); changed['distros'][key][field]['value'] = value
            with self.assertRaises(ValueError): h.preserved(before, changed)

    def test_preexisting_stage_or_path_or_name_never_submits_import(self):
        for conflict in ('stage', 'path', 'name'):
            with self.subTest(conflict=conflict), tempfile.TemporaryDirectory() as directory:
                root = Path(directory); install = root / 'new-distro'; before = state()
                if conflict == 'stage': h.p.publish(root / 'import-started.json', {})
                if conflict == 'path': install.mkdir()
                if conflict == 'name': before = state(True)
                with patch.object(h.p, 'OUTPUT', root), patch.object(h.p, 'INSTALL', install), \
                        patch.object(h.p, 'sha', return_value=h.PREPARED_SHA), \
                        patch.object(h, 'snapshot', return_value=before), patch.object(h, 'command') as command:
                    with self.assertRaises(ValueError): h.run('import')
                    command.assert_not_called()

    def test_boot_requires_successful_import_and_unchanged_existing_registry(self):
        for exit_code in (None, 1):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory); h.p.publish(root / 'import-command.json', dict(exit=exit_code))
                with patch.object(h.p, 'OUTPUT', root), patch.object(h.p, 'sha', return_value=h.PREPARED_SHA), \
                        patch.object(h, 'snapshot', return_value=state(True)), patch.object(h, 'command') as command:
                    with self.assertRaises(ValueError): h.run('boot')
                    command.assert_not_called()

    def test_exact_import_argv_and_no_global_or_existing_distro_mutation(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); install = root / 'new-distro'
            after = state(True); after['distros']['new-guid']['BasePath']['value'] = str(install)
            with patch.object(h.p, 'OUTPUT', root), patch.object(h.p, 'INSTALL', install), \
                    patch.object(h.p, 'sha', return_value=h.PREPARED_SHA), \
                    patch.object(h, 'snapshot', side_effect=[state(), after]), patch.object(h, 'command') as command:
                h.run('import')
                command.assert_called_once_with(['wsl.exe', '--import', h.p.DISTRO, str(install),
                    str(root / 'debian-prepared-v2.tar'), '--version', '2'], 180, 'import-command')
                result = json.loads((root / 'import-result.json').read_text())
                self.assertFalse(result['original_delivery']); self.assertFalse(result['writers_cessation'])

    def test_timeout_receipt_keeps_partial_output_and_never_resubmits(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with patch.object(h.p, 'OUTPUT', root), patch.object(h.subprocess, 'run',
                    side_effect=subprocess.TimeoutExpired(['synthetic'], 1, output=b'partial')) as child:
                with self.assertRaisesRegex(ValueError, 'do not resubmit'): h.command(['synthetic'], 1, 'first')
                child.assert_called_once()
            self.assertEqual((root / 'first.stdout.log').read_bytes(), b'partial')
            result = json.loads((root / 'first.json').read_text())
            self.assertIsNone(result['exit']); self.assertIn('no resubmit', result['failure'])


if __name__ == '__main__': unittest.main(verbosity=2)
