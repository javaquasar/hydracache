"""Registry writer regression controls; mocks and owned regular files only."""
from contextlib import contextmanager
import errno
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

import capture as cap

PATH = Path('/not-a-live-tracefs/owned-registry')
GROUP = 'hc074clock_' + 'c' * 32


@contextmanager
def backend(command):
    calls = Mock()
    with patch.object(Path, 'open', side_effect=AssertionError('High-level append stream is forbidden')), \
         patch.object(cap.os, 'lseek', side_effect=AssertionError('No registry seek allowed')) as seek, \
         patch.object(cap.os, 'open', return_value=123) as opening, \
         patch.object(cap.os, 'write', return_value=len((command + '\n').encode('ascii'))) as writing, \
         patch.object(cap.os, 'close') as closing:
        for name, method in (('open', opening), ('write', writing), ('close', closing)):
            calls.attach_mock(method, name)
        yield calls, opening, writing, closing, seek


class RegistryWriter(unittest.TestCase):
    def test_complete_command_uses_no_seek_create_or_truncate(self):
        command = cap.definitions(GROUP)[0]
        with backend(command) as (calls, opening, writing, closing, seek):
            cap.write(PATH, command, append=True)
            opening.assert_called_once_with(PATH, os.O_WRONLY | os.O_APPEND | os.O_CLOEXEC)
            self.assertFalse(opening.call_args.args[1] & (os.O_CREAT | os.O_TRUNC))
            writing.assert_called_once_with(123, (command + '\n').encode('ascii'))
            closing.assert_called_once_with(123)
            seek.assert_not_called()
            self.assertEqual([c[0] for c in calls.mock_calls], ['open', 'write', 'close'])

    def test_all_exact_registration_and_deletion_commands(self):
        planned = cap.definitions(GROUP)
        commands = planned + ['-:' + cap.key(s) for s in reversed(planned)]
        for command in commands:
            with self.subTest(command=command), backend(command) as (_, _, writing, closing, _):
                cap.write(PATH, command, append=True)
                writing.assert_called_once_with(123, (command + '\n').encode('ascii'))
                closing.assert_called_once_with(123)

    def test_short_and_zero_write_refuse_without_resubmission(self):
        for count in (0, 2):
            with self.subTest(count=count), backend('owned-command') as (_, _, writing, closing, _):
                writing.return_value = count
                with self.assertRaises(OSError) as caught:
                    cap.write(PATH, 'owned-command', append=True)
                self.assertEqual(caught.exception.errno, errno.EIO)
                self.assertEqual(caught.exception.filename, str(PATH))
                writing.assert_called_once()
                closing.assert_called_once_with(123)

    def test_open_refusal_never_writes_or_closes_unowned_fd(self):
        with backend('owned-command') as (_, opening, writing, closing, _):
            opening.side_effect = OSError(errno.EINVAL, 'injected open refusal')
            with self.assertRaises(OSError) as caught:
                cap.write(PATH, 'owned-command', append=True)
            self.assertEqual(caught.exception.errno, errno.EINVAL)
            self.assertEqual(caught.exception.filename, str(PATH))
            opening.assert_called_once()
            writing.assert_not_called()
            closing.assert_not_called()

    def test_write_refusal_or_interrupt_closes_once_without_retry(self):
        for code in (errno.EIO, errno.EINTR):
            with self.subTest(errno=code), backend('owned-command') as (_, _, writing, closing, _):
                writing.side_effect = OSError(code, 'injected write refusal')
                with self.assertRaises(OSError) as caught:
                    cap.write(PATH, 'owned-command', append=True)
                self.assertEqual(caught.exception.errno, code)
                writing.assert_called_once()
                closing.assert_called_once_with(123)

    def test_close_refusal_has_exact_path_and_never_retries_close(self):
        with backend('owned-command') as (_, _, writing, closing, _):
            closing.side_effect = OSError(errno.EIO, 'injected close refusal')
            with self.assertRaises(OSError) as caught:
                cap.write(PATH, 'owned-command', append=True)
            self.assertEqual(caught.exception.filename, str(PATH))
            self.assertIn('close refusal', str(caught.exception))
            writing.assert_called_once()
            closing.assert_called_once_with(123)

    def test_first_write_error_is_not_replaced_by_close_error(self):
        with backend('owned-command') as (_, _, writing, closing, _):
            writing.side_effect = OSError(errno.ENOSPC, 'first write refusal')
            closing.side_effect = OSError(errno.EIO, 'later close refusal')
            with self.assertRaises(OSError) as caught:
                cap.write(PATH, 'owned-command', append=True)
            self.assertEqual(caught.exception.errno, errno.ENOSPC)
            self.assertIn('first write refusal', str(caught.exception))
            self.assertIn('later close refusal', '\n'.join(caught.exception.__cause__.__notes__))
            closing.assert_called_once_with(123)

    def test_invalid_or_multiple_commands_refuse_before_open(self):
        for command in ('', 'one\ntwo', 'one\rtwo', 'non-ascii-\u20ac'):
            with self.subTest(command=command), patch.object(cap.os, 'open') as opening:
                with self.assertRaises((ValueError, UnicodeEncodeError)):
                    cap.write(PATH, command, append=True)
                opening.assert_not_called()

    def test_owned_regular_file_preserves_existing_bytes_and_missing_file_refuses(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'owned-registry'
            path.write_bytes(b'foreign-existing-definition\n')
            cap.write(path, 'owned-definition', append=True)
            cap.write(path, '-:owned-definition', append=True)
            self.assertEqual(path.read_bytes(), b'foreign-existing-definition\nowned-definition\n-:owned-definition\n')
            missing = Path(folder) / 'must-not-create'
            with self.assertRaises(FileNotFoundError):
                cap.write(missing, 'owned-definition', append=True)
            self.assertFalse(missing.exists())


if __name__ == '__main__':
    unittest.main()
