from pathlib import Path
import importlib.util,json,unittest
spec=importlib.util.spec_from_file_location('open_review',Path(__file__).with_name('review.py'))
api=importlib.util.module_from_spec(spec);spec.loader.exec_module(api)
def synthetic(phase='prototype',ratio=0.9):
 result=[]
 for cohort in range(8):
  lines=['HC074-NAMESPACE-OPEN-ORIGIN '+json.dumps(dict(cohort=cohort,phase=phase,uid=1000,samples=16,iterations=1024,warmups=32,product=False,admission=False))]
  for case in api.CASES:
   for sample in range(16):
    for slot,variant in enumerate(api.common.ORDERS[(sample+cohort)%8]):
     arm=api.ARMS[variant];cpu=int(100000*ratio) if arm=='prototype' or (phase=='installed' and arm=='runtime') else 100000
     lines.append('HC074-NAMESPACE-OPEN-COST '+json.dumps(dict(cohort=cohort,phase=phase,case=case,sample=sample,slot=slot,variant=arm,
      calls=1024,opens_per_call=1,filesystem_checks=1,identity_checks=1,cpu_ns=cpu,wall_ns=110000,product=False,admission=False)))
  lines.append('test result: ok. 1 passed; 0 failed; 0 ignored;');result.append(('\n'.join(lines)+'\n').encode())
 return result
class Controls(unittest.TestCase):
 def test_synthetic_phases_are_never_admission(self):
  for phase in ['prototype','installed']:
   value=api.review(synthetic(phase),phase);self.assertTrue(value['cpu_screen_passed']);self.assertFalse(value['admission'])
 def test_changed_work_order_types_and_clocks_refuse(self):
  raw=synthetic()[0]
  for a,b in [('"calls": 1024','"calls": 1023'),('"slot": 0','"slot": 1'),('"cohort": 0','"cohort": false'),
   ('"filesystem_checks": 1','"filesystem_checks": 0'),('"identity_checks": 1','"identity_checks": 0'),
   ('"cpu_ns": 100000','"cpu_ns": 0'),('"product": false','"product": true')]:
   with self.assertRaises(ValueError):api.parse(raw.replace(a.encode(),b.encode(),1),0,'prototype')
 def test_missing_duplicate_or_wrong_phase_refuse(self):
  with self.assertRaises(ValueError):api.review(synthetic()[:-1],'prototype')
  with self.assertRaises(ValueError):api.parse(synthetic()[0]*2,0,'prototype')
  with self.assertRaises(ValueError):api.parse(synthetic()[0],0,'installed')
 def test_effect_below_preregistered_floor_is_negative(self):
  self.assertFalse(api.review(synthetic(ratio=0.99),'prototype')['cpu_screen_passed'])
 def test_integrated_image_cannot_supply_baseline_calibration(self):
  rows=[raw.replace(b'"phase": "installed"',b'"phase": "prototype"') for raw in synthetic('installed')]
  self.assertFalse(api.review(rows,'prototype')['cpu_screen_passed'])
if __name__=='__main__':unittest.main()
