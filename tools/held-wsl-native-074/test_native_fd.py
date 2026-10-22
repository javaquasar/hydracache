"""Synthetic wrapper controls; actual native bus behavior is a separate lane."""
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import native_fd as n


class Controls(unittest.TestCase):
    def test_busctl_payload_has_an_option_terminator_before_helper_arguments(self):
        source = Path(n.__file__).with_suffix('.sh').read_text()
        self.assertIn('250ms -- "$@"', source)

    def test_argv_has_only_the_owned_distro_and_fixed_script(self):
        nonce = '74' * 16
        self.assertEqual(n.argv(nonce), ['wsl.exe', '-d', 'HydraCache074Native', '-u', 'root',
            '--cd', '/', '--exec', '/bin/bash', n.SCRIPT, nonce])
        for bad in ('', 'x' * 32, '../escape', nonce + '; command', nonce.upper() + 'A'):
            with self.assertRaises(ValueError): n.argv(bad)

    def test_existing_outcome_never_opens_host_or_resubmits(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(n, 'OUTPUT', Path(directory)), patch.object(n.host, 'snapshot') as snapshot, \
                    patch.object(n.subprocess, 'run') as child:
                with self.assertRaisesRegex(ValueError, 'Never resume'): n.run()
                snapshot.assert_not_called(); child.assert_not_called()

    def test_fixed_shell_semantics_never_look_like_original_transport(self):
        source = Path(n.__file__).with_suffix('.sh').read_text()
        self.assertEqual(source.count(' StartTransientUnit '), 1)
        for token in ('ExtraFileDescriptors', "'a(hs)' 2 3 role 4 empty", 'StandardOutputFileDescriptor h 5',
                'StandardErrorFileDescriptor h 6', 'test "${LISTEN_PID:-}" = "$$"', 'GetConnectionUnixProcessID',
                'GetConnectionUnixUser', 'NoNewPrivileges b true', 'Restart s no', 'StopUnit ss "$unit" fail',
                'cleanup-uncertain-do-not-resend', 'native-fd-mechanics-passed-not-original-delivery'):
            self.assertIn(token, source)
        for forbidden in ('systemctl stop', 'systemctl restart', 'ResetFailedUnit', 'StartUnit', 'date -s', 'timedatectl set'):
            self.assertNotIn(forbidden, source)


if __name__ == '__main__': unittest.main(verbosity=2)
