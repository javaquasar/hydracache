"""Offline exact recovery boundary; never a measurement-frame exception."""
import unittest
import recover as r

OLD='54 28 0:47 / /run/user/1000 rw,nosuid,nodev,relatime shared:186 - tmpfs tmpfs rw,mode=700,uid=1000,gid=1000'
NEW='53 28 0:42 / /run/user/1000 rw,nosuid,nodev,relatime shared:149 - tmpfs tmpfs rw,mode=700,uid=1000,gid=1000'
OTHER='30 1 9:2 / / rw,relatime shared:1 - ext4 /dev/md2 rw'


class Recovery(unittest.TestCase):
    def test_exact_observed_leaf_replacement_retains_failed_frame(self):
        self.assertFalse(r.mount_replacement(OTHER+'\n'+OLD,OTHER+'\n'+NEW)['measurement_frame_pass'])

    def test_other_mount_change_refused(self):
        with self.assertRaises(RuntimeError):r.mount_replacement(OTHER+'\n'+OLD,OTHER.replace('9:2','9:3')+'\n'+NEW)

    def test_options_parent_path_nested_or_type_change_refused(self):
        for text in (NEW.replace('mode=700','mode=755'),NEW.replace('53 28','53 29'),NEW.replace('tmpfs tmpfs','ext4 other'),NEW+'\n'+NEW.replace('/run/user/1000','/run/user/1000/nested')):
            with self.assertRaises(RuntimeError):r.mount_replacement(OLD,text)

    def test_missing_duplicate_unchanged_mount_refused(self):
        for text in ('',NEW+'\n'+NEW,OLD):
            with self.assertRaises(RuntimeError):r.mount_replacement(OLD,text)

    def test_protected_host_drift_refused(self):
        before=dict(mounts=OLD,supervisor='inactive',boot='same',protected={'image':'a'})
        after=dict(before,mounts=NEW,protected={'image':'b'})
        with self.assertRaises(RuntimeError):r.host_review(before,after)

    def test_running_or_restartable_unit_refused(self):
        good=dict(MainPID='0',ActiveState='failed',SubState='failed',ControlGroup='',Restart='no',KillMode='control-group',ExecMainStatus='1')
        r.stopped_unit(good)
        for k,v in (('MainPID','3'),('ActiveState','active'),('ControlGroup','/a'),('Restart','always'),('KillMode','process')):
            with self.assertRaises(RuntimeError):r.stopped_unit(dict(good,**{k:v}))


if __name__=='__main__':unittest.main()
