import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
import check as c


class Controls(unittest.TestCase):
    def test_fixed_lanes_never_select_the_ignored_matrix_or_a_unit(self):
        for lane in ('host', '194', '188', 'metadata'):
            for name, argv in c.commands(lane):
                self.assertFalse(set(argv) & {'--ignored', 'ssh', 'systemctl', 'dispatch', 'bench'})
                if name == 'process': self.assertIn('held_activation_environment', argv)
                if name == 'binary': self.assertIn('diagnostic_held_launcher', argv)
        with self.assertRaises(ValueError): c.commands('server')

    def test_existing_lane_refuses_without_subprocess_or_resume(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(c, 'OUTPUT', Path(tmp)):
            (Path(tmp) / '194').mkdir()
            with patch.object(c.logger.subprocess, 'run') as child:
                with self.assertRaises(FileExistsError): c.run('194')
                child.assert_not_called()

    def test_commands_source_and_output_are_passed_to_the_unchanged_logger(self):
        with patch.object(c.logger, 'run') as run:
            c.run('188')
            run.assert_called_once_with('188')
            self.assertEqual(c.logger.FILES, c.FILES)
            self.assertEqual(c.logger.OUTPUT, c.OUTPUT)
            self.assertIs(c.logger.commands, c.commands)


if __name__ == '__main__': unittest.main()
