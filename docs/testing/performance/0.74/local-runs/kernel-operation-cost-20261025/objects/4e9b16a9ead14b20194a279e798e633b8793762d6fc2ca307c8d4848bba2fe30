from pathlib import Path
import importlib.util,json,unittest
spec=importlib.util.spec_from_file_location('operation_review',Path(__file__).with_name('review.py'))
api=importlib.util.module_from_spec(spec);spec.loader.exec_module(api)
def synthetic():
 result=[]
 for cohort in range(8):
  lines=['HC074-OPERATION-ORIGIN '+json.dumps(dict(cohort=cohort,uid=1000,samples=16,iterations=256,warmups=16,product=False,admission=False))]
  for index,case in enumerate(api.CASES):
   for stage_index,stage in enumerate(api.STAGES):
    for sample in range(16):
     for slot,variant in enumerate(api.common.ORDERS[(sample+cohort)%8]):
      calls=api.COUNTS[int(index!=0)][stage_index]*256 if variant==3 else 0
      lines.append('HC074-OPERATION-COST '+json.dumps(dict(cohort=cohort,case=case,stage=stage,sample=sample,slot=slot,
       variant=api.ARMS[variant],calls=256,operation_calls=calls,operation_cpu_ns=calls*200,clock_failed=False,
       cpu_ns=120000 if variant==3 else 100000,wall_ns=130000,product=False,admission=False)))
  lines.append('test result: ok. 1 passed; 0 failed; 0 ignored;')
  result.append(('\n'.join(lines)+'\n').encode())
 return result
class Controls(unittest.TestCase):
 def test_exact_shape_preserves_nonadditive_scope(self):
  value=api.review(synthetic())
  self.assertTrue(value['calibration_passed']);self.assertFalse(value['admission'])
  self.assertFalse(value['empty_control_subtracted']);self.assertFalse(value['spans_additive'])
  self.assertEqual(value['cases']['boot']['identity']['selected_cpu_ns_per_operation'],None)
  self.assertEqual(value['cases']['user']['identity']['selected_cpu_ns_per_operation'],200)
 def test_changed_counts_order_types_or_clocks_refuse(self):
  raw=synthetic()[0]
  for old,new in [('"calls": 256','"calls": 255'),('"slot": 0','"slot": 2'),
   ('"cohort": 0','"cohort": false'),('"operation_calls": 1024','"operation_calls": 1023'),
   ('"operation_cpu_ns": 204800','"operation_cpu_ns": 0'),('"cpu_ns": 100000','"cpu_ns": 0'),
   ('"clock_failed": false','"clock_failed": true'),('"admission": false','"admission": true')]:
   with self.assertRaises(ValueError):api.parse(raw.replace(old.encode(),new.encode(),1),0)
 def test_missing_repeated_phase_or_verdict_refuse(self):
  rows=synthetic()
  with self.assertRaises(ValueError):api.review(rows[:-1])
  with self.assertRaises(ValueError):api.parse(rows[0]+rows[0],0)
  with self.assertRaises(ValueError):api.parse(rows[0].replace(b'test result: ok.',b'test result: FAILED.'),0)
 def test_calibration_failure_cannot_become_a_speedup(self):
  rows=synthetic()
  changed=[]
  for raw in rows:
   lines=[]
   for line in raw.decode().splitlines():
    if line.startswith('HC074-OPERATION-COST '):
     row=json.loads(line.split(' ',1)[1])
     if row['variant']=='runtime':row['cpu_ns']=50000
     line='HC074-OPERATION-COST '+json.dumps(row)
    lines.append(line)
   changed.append(('\n'.join(lines)+'\n').encode())
  value=api.review(changed)
  self.assertFalse(value['calibration_passed']);self.assertFalse(value['performance_improvement_proven'])
if __name__=='__main__':unittest.main()
