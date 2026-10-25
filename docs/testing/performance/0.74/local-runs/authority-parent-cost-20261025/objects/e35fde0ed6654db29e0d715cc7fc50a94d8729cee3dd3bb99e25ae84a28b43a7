from pathlib import Path
import importlib.util,json,unittest
spec=importlib.util.spec_from_file_location('before_review',Path(__file__).with_name('review.py'))
api=importlib.util.module_from_spec(spec);spec.loader.exec_module(api)


def synthetic(phase='prototype'):
    result=[]
    for cohort in range(8):
        rows=['HC074-PARENT-ORIGIN '+json.dumps(dict(cohort=cohort,phase=phase,samples=16,
            iterations=1024,uid=1000,warmups=32,product=False,admission=False))]
        for payload,(length,budget) in api.PAYLOADS.items():
            for sample in range(16):
                for slot,variant in enumerate(api.ORDERS[(sample+cohort)%8]):
                    arm=api.ARMS[variant]
                    cpu=90000 if arm=='prototype' or (phase=='installed' and arm=='runtime') else 100000
                    rows.append('HC074-PARENT-COST '+json.dumps(dict(cohort=cohort,phase=phase,payload=payload,
                        sample=sample,slot=slot,variant=arm,calls=1024,metadata_observations=4,
                        payload_bytes=length,budget=budget,cpu_ns=cpu,wall_ns=110000,product=False,admission=False)))
        rows.append('test result: ok. 1 passed; 0 failed; 0 ignored;')
        result.append(('\n'.join(rows)+'\n').encode())
    return result


class Controls(unittest.TestCase):
    def test_synthetic_phases_never_admit(self):
        for phase in ['prototype','installed']:
            r=api.review(synthetic(phase),phase)
            self.assertTrue(r['cpu_screen_passed']);self.assertFalse(r['admission'])

    def test_changed_work_order_clock_and_flags_refuse(self):
        for a,b in [('"calls": 1024','"calls": 1023'),('"slot": 0','"slot": 9'),
                    ('"metadata_observations": 4','"metadata_observations": 3'),('"budget": 65','"budget": 64'),
                    ('"cpu_ns": 90000','"cpu_ns": 0'),('"cohort": 0','"cohort": false'),
                    ('"product": false','"product": true')]:
            rows=synthetic();rows[0]=rows[0].replace(a.encode(),b.encode(),1)
            with self.assertRaises(ValueError):api.review(rows,'prototype')

    def test_missing_or_duplicate_data_refuse(self):
        with self.assertRaises(ValueError):api.review(synthetic()[:-1],'prototype')
        rows=synthetic();rows[0]+=rows[0].splitlines()[1]+b'\n'
        with self.assertRaises(ValueError):api.review(rows,'prototype')

    def test_wrong_phase_or_verdict_refuse(self):
        with self.assertRaises(ValueError):api.review(synthetic(),'installed')
        rows=synthetic();rows[0]=rows[0].replace(b'1 passed;',b'0 passed;')
        with self.assertRaises(ValueError):api.review(rows,'prototype')

    def test_negative_cpu_is_retained(self):
        rows=[r.replace(b'"cpu_ns": 90000',b'"cpu_ns": 101000') for r in synthetic()]
        self.assertFalse(api.review(rows,'prototype')['cpu_screen_passed'])

    def test_failed_runtime_calibration_refuses(self):
        # Strong drift in every process, only runtime arm.
        rows=[]
        for raw in synthetic():
            lines=[]
            for line in raw.decode().splitlines():
                if line.startswith('HC074-PARENT-COST '):
                    body=json.loads(line.split(' ',1)[1])
                    if body['variant']=='runtime':body['cpu_ns']=110000
                    line='HC074-PARENT-COST '+json.dumps(body)
                lines.append(line)
            rows.append(('\n'.join(lines)+'\n').encode())
        self.assertFalse(api.review(rows,'prototype')['cpu_screen_passed'])
if __name__=='__main__':unittest.main()
