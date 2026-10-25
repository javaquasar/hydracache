"""Mutation controls are offline; no new workload or host action."""
import hashlib
import json
from pathlib import Path
import unittest
import verify as replay

HERE=Path(__file__).resolve().parent


def changed(logical,mutate):
    index=json.loads((HERE/'index.json').read_bytes())
    old=index['members'][logical]
    value=json.loads((HERE/'objects'/old['sha256']).read_bytes())
    mutate(value)
    raw=json.dumps(value,separators=(',',':')).encode()
    pin=hashlib.sha256(raw).hexdigest()
    index['members'][logical]=dict(sha256=pin,bytes=len(raw))
    def read(name):
        if name=='index.json':return json.dumps(index).encode()
        if name=='objects/'+pin:return raw
        return (HERE/name).read_bytes()
    return read


class Controls(unittest.TestCase):
    def test_complete_fixed_gate_replays(self):
        result=replay.verify(lambda name:(HERE/name).read_bytes())
        self.assertTrue(result['terminal_rejection'])
        self.assertEqual(result['attempts'],240)
        self.assertFalse(result['candidate_admitted'])

    def test_raw_mutation_is_not_accepted(self):
        index=json.loads((HERE/'index.json').read_bytes())
        target='objects/'+index['members']['stage/239.stdout.log']['sha256']
        def read(name):
            raw=(HERE/name).read_bytes()
            return raw+b' ' if name==target else raw
        with self.assertRaisesRegex(ValueError,'raw digest'):replay.verify(read)

    def test_populated_writer_is_not_a_retirement(self):
        read=changed('stage/239.retired.json',lambda d:d['events'].update(populated='1'))
        with self.assertRaisesRegex(ValueError,'populated writer'):replay.verify(read)

    def test_rewritten_first_refusal_is_rejected(self):
        read=changed('windows/get-owner-v2-semantic-compiled-first/result.json',lambda d:d.update(exit=0))
        with self.assertRaisesRegex(ValueError,'first Windows'):replay.verify(read)

    def test_secure_one_megabyte_cannot_round_up_to_the_floor(self):
        gate=replay.module('allocation_gate')
        ci=gate.interval([.80023]*5)
        self.assertGreater(ci['lower_95'],.8)
        read=changed('analysis-first.json',lambda d:d['rejected_cells'].remove('resp2-mtls-get-1m-p10'))
        with self.assertRaisesRegex(ValueError,'first fixed-estimator'):replay.verify(read)

    def test_wire_placement_drift_survives_hash_rewrite(self):
        read=changed('stage/120.stdout.log',lambda d:d['report']['observation']['observed'].update(physical_connections=8))
        with self.assertRaisesRegex(RuntimeError,'wire population'):replay.verify(read)

    def test_source_drift_survives_hash_rewrite(self):
        read=changed('stage/120.stdout.log',lambda d:d.update(source_commit_from_coordinator='0'*40))
        with self.assertRaisesRegex(RuntimeError,'pin mismatch'):replay.verify(read)

    def test_last_pair_cannot_be_omitted(self):
        index=json.loads((HERE/'index.json').read_bytes())
        del index['members']['stage/239.stdout.log']
        def read(name):return json.dumps(index).encode() if name=='index.json' else (HERE/name).read_bytes()
        with self.assertRaises(KeyError):replay.verify(read)


if __name__=='__main__':unittest.main()
