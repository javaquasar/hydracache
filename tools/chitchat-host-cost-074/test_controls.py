"""Offline controls; no SSH, process launch, clock read or host mutation."""
import ast
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
import types
import host
import remote
import prepare

ROOT=Path(__file__).resolve().parents[2]


class Controls(unittest.TestCase):
    def state(self):return json.loads((prepare.OUT/'preflight.stdout.log').read_bytes())

    def test_current_read_only_admission(self):host.admit(self.state())

    def test_campaign_and_context_refusals(self):
        for marker in host.MARKERS:
            row=self.state();row['markers'][marker]=True
            with self.assertRaises(ValueError):host.admit(row)
        row=self.state();row['leftovers']=['/tmp/hc074-owned-root-other']
        with self.assertRaises(ValueError):host.admit(row)

    def test_host_pins_abi_supervisor(self):
        for field,value in (('libc','glibc 2.40'),('machine','aarch64'),('page_size',65536),('supervisor','ActiveState=inactive\nSubState=dead\nMainPID=0')):
            row=self.state();row[field]=value
            with self.assertRaises(ValueError):host.admit(row)
        for path in host.PROTECTED:
            row=self.state();row['protected'][path]='0'*64
            with self.assertRaises(ValueError):host.admit(row)

    def test_fixed_control_no_numerical_dispatch(self):
        for v in ('B','P','Q'):
            argv=remote.fixed_argv(Path('/tmp/fixture'),v)
            self.assertEqual(argv[-1],'control');self.assertEqual(argv[:2],['/usr/bin/env','-i'])
            self.assertNotIn('cpu',argv);self.assertNotIn('memory',argv)
        for bad in ('cpu','memory','../B','B control'):
            with self.assertRaises(ValueError):remote.fixed_argv(Path('/tmp/fixture'),bad)

    def test_exact_functions_without_rewrite(self):
        for path,name in (('tools/windows-workspace-preflight-074/owned.py','_owned'),('tools/chitchat-cost-074/analyze.py','validate')):
            raw=(ROOT/path).read_bytes();text=raw.decode();node=next(n for n in ast.parse(text).body if isinstance(n,ast.FunctionDef) and n.name==name)
            self.assertEqual(prepare.function(raw,name).decode(),ast.get_source_segment(text,node)+'\n')
        raw=(ROOT/'tools/windows-workspace-preflight-074/owned.py').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(),'e7c5a3c0c4fea90f39772512c4ad96c60693fc0b482092a608dcf039e151eade')

    def test_input_inventory_mutations(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'target') as tmp:
            root=Path(tmp);path=root/'one';path.write_bytes(b'first')
            self.assertEqual(remote.inventory(root),{'one':remote.sha(path)})
            path.write_bytes(b'changed');self.assertNotEqual(remote.inventory(root)['one'],hashlib.sha256(b'first').hexdigest())
            (root/'extra').write_bytes(b'extra');self.assertEqual(len(remote.inventory(root)),2)
            with self.assertRaises(ValueError):remote.verify(root,{'files':{'one':'0'*64}})

    def test_first_receipt_never_overwritten(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'target') as tmp:
            path=Path(tmp)/'first.json';remote.put(path,dict(first=74))
            with self.assertRaises(FileExistsError):remote.put(path,dict(first=75))
            self.assertEqual(json.loads(path.read_bytes()),dict(first=74))

    def test_carrier_keeps_limits_no_service_or_lock_mutation(self):
        raw=(Path(__file__).parent/'remote.py').read_text()
        for value in ('+1100','bound=1200','deadline=float(deadline)','cohort_dispatched=False','sys.dont_write_bytecode=True'):
            self.assertIn(value,raw)
        for value in ('systemctl stop','systemctl restart','flock(', 'clock_settime','timedatectl','malloc_trim','nice -n'):
            self.assertNotIn(value,raw)

    def test_same_clock_refusal_is_not_rescued_by_host_adapter(self):
        root=prepare.OUT/'prepared'
        validate=remote.validator(root)
        original=ROOT/'target/chitchat-cost-20261029-r1/control-B/native/observation/stdout.log'
        rows=[s.removeprefix('HC074_CLEAN_COST:') for s in original.read_text().splitlines() if s.startswith('HC074_CLEAN_COST:')]
        self.assertEqual(len(rows),1)
        with self.assertRaisesRegex(ValueError,'no suspend/raw-clock discrepancy'):validate(json.loads(rows[0]),'B','control')

    def test_dispatch_latches_and_detached_owner_are_fixed(self):
        raw=(Path(__file__).parent/'dispatch.py').read_text()
        for marker in ("OUT/'dispatch-once.json'",'nohup /usr/bin/env -i',"'stage'",'tarfile.open(archive,\'x:gz\')','filter="data"'):
            self.assertIn(marker,raw)
        self.assertLess(raw.index("OUT/'dispatch-once.json'"),raw.index("call('dispatch-preflight'"))
        self.assertNotIn('systemctl stop',raw);self.assertNotIn('clock_settime',raw)
        self.assertNotIn('retry=True',raw)

    def test_complete_healthy_body_covers_validator_global_closure(self):
        body=json.loads((prepare.OUT/'collection/control-B/raw-body.json').read_bytes())
        remote.validator(prepare.OUT/'prepared')(body,'B','control')
        self.assertFalse(json.loads((prepare.OUT/'collection/result.json').read_bytes())['complete'])
        self.assertEqual(json.loads((prepare.OUT/'collection/session-result.json').read_bytes())['first_refusal']['reason'],"name 'FIELDS' is not defined")

    def test_validator_globals_are_only_complete_literal_data(self):
        raw=(ROOT/'tools/chitchat-cost-074/analyze.py').read_bytes();values=remote.validation_constants(raw)
        self.assertEqual(set(values),{'FIELDS','HEAP'});self.assertEqual(len(values['FIELDS']),4);self.assertEqual(len(values['HEAP']),10)
        for bad in (b'FIELDS=("rss",)',b'FIELDS=("rss",)\nHEAP=__import__("os").system("false")',b'FIELDS=("rss",)\nHEAP=("arena",)\nFIELDS=("pss",)'):
            with self.assertRaises((ValueError,TypeError)):remote.validation_constants(bad)


if __name__=='__main__':unittest.main()
