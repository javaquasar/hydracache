import copy
import json
import unittest
import candidate_capture as candidate
import capture


def fixture():
    rows=[]
    for block in range(6):
        for path in capture.PATHS:
            for slot,selected in enumerate((False,True,True,False) if block%2==0 else (True,False,False,True)):
                rows.append(dict(block=block,path=path,slot=slot,candidate=selected,iterations=32,accepted=32,wall_ns=320,parent_cpu_ns=0))
    return rows


def encode(rows):
    return ('\n'.join('HC074_CANONICAL '+json.dumps(row) for row in rows)+
        '\nHC074_CANONICAL_DONE {"blocks":6,"samples":72,"warmup":false,"allocator_interposition":false,"installed":false,"product":false,"package_replay":false}\n').encode()


class Controls(unittest.TestCase):
    def test_complete_not_accepted_and_not_allocation_evidence(self):
        result=candidate.review(encode(fixture()))
        self.assertEqual(len(result['samples']),72)
        self.assertFalse(result['allocation_measured'])
        self.assertFalse(result['accepted_for_integration'])
        self.assertFalse(result['independent_process_replicates'])

    def test_incomplete_order_result_iteration_and_boolean_drift_refuse(self):
        for mode in range(7):
            rows=copy.deepcopy(fixture())
            if mode==0:rows.pop()
            elif mode==1:rows.append(rows[0])
            elif mode==2:rows[0],rows[1]=rows[1],rows[0]
            elif mode==3:rows[0]['slot']=False
            elif mode==4:rows[0]['iterations']=31
            elif mode==5:rows[0]['accepted']=31
            else:rows[0]['wall_ns']=-1
            with self.assertRaises(ValueError):candidate.review(encode(rows))


if __name__=='__main__':unittest.main()
