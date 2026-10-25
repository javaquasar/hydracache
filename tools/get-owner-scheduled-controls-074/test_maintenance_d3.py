"""Offline coordinator refusal controls; no host, service or workload actions."""
from pathlib import Path, PurePosixPath
import importlib.util
import sys
import unittest
from unittest.mock import Mock, patch

spec=importlib.util.spec_from_file_location('maintenance_d3',Path(__file__).with_name('maintenance_d3.py'))
m=importlib.util.module_from_spec(spec)
if sys.platform=='win32':
    with patch.dict(sys.modules,{'fcntl':Mock()}):spec.loader.exec_module(m)
else:spec.loader.exec_module(m)


class Refusals(unittest.TestCase):
    def test_deadline_is_absolute_not_a_fresh_relative_allowance(self):
        with patch.object(m,'command') as call:
            m.timer_install({'unit':'hc074-get-owner-d3-x.service'},7,PurePosixPath('/tmp/hc074-get-owner-d3-x'),123456789000)
        argv=call.call_args.args[0]
        self.assertIn('--on-boot=123.456789000s',argv)
        self.assertFalse(any(s.startswith('--on-active=') for s in argv))
        self.assertEqual(argv[-2:],['/tmp/hc074-get-owner-d3-x','7'])

    def test_timer_gc_is_typed_not_a_blanket_stop_exception(self):
        for state,code,passes in [
            ('LoadState=not-found\nActiveState=inactive\nMainPID=0\n',5,True),
            ('LoadState=loaded\nActiveState=inactive\nMainPID=0\n',0,True),
            ('LoadState=loaded\nActiveState=active\nMainPID=123\n',0,False),
            ('LoadState=loaded\nActiveState=inactive\nMainPID=0\n',5,False),
            ('',5,False),
        ]:
            stopped=Mock(returncode=code)
            shown=Mock(stdout=state.encode())
            with self.subTest(state=state,code=code),patch.object(m.subprocess,'run',side_effect=[stopped,shown]*2):
                if passes:m.timer_stop({'unit':'hc074-get-owner-d3-x.service'},0)
                else:
                    with self.assertRaises(RuntimeError):m.timer_stop({'unit':'hc074-get-owner-d3-x.service'},0)

    def test_missing_or_replaced_cgroup_never_counts_as_empty(self):
        with patch.object(m,'group_identity',side_effect=OSError(19,'ENODEV')):
            with self.assertRaises(OSError):m.empty_group(Path('/original'),[1,2])
        with patch.object(m,'group_identity',return_value=[1,3]):
            with self.assertRaisesRegex(RuntimeError,'inode changed'):m.empty_group(Path('/original'),[1,2])

    def test_populated_writer_refuses_retirement(self):
        with patch.object(m,'group_identity',return_value=[1,2]),patch.object(Path,'read_text',return_value='populated 1\nfrozen 0\n'):
            with self.assertRaisesRegex(RuntimeError,'remain populated'):m.empty_group(Path('/original'),[1,2])

    def test_receipt_pins_and_missing_measurement_refuse(self):
        with self.assertRaisesRegex(RuntimeError,'pin mismatch'):
            m.validate_receipt({'schema_version':1},{'receipt_pins':{'schema_version':2}})
        with self.assertRaisesRegex(RuntimeError,'invalid allocation receipt'):
            m.validate_receipt({'error':None,'report':{}},{'receipt_pins':{}})

    def test_active_stable_service_prevents_external_fence_acquisition(self):
        state=dict(ActiveState='active',SubState='running',MainPID='8839',NRestarts='0')
        # Use a real absent relative directory; all host/effect operations mocked.
        with patch.object(m,'own_group',return_value=Path('/owned')),patch.object(m,'static_frame'),patch.object(m,'group_identity',return_value=[1,2]),patch.object(m,'empty_group'),patch.object(m,'service_state',return_value=state),patch.object(m,'open_fence') as fence,patch.object(m,'publish') as publish:
            m.post({'unit':'fixture.service','supervisor':'stable.service','supervisor_before':state},Path('absent-offline-fixture'))
            fence.assert_not_called()
            self.assertTrue(publish.call_args.args[2]['maintenance_never_entered'])


if __name__=='__main__':unittest.main()
