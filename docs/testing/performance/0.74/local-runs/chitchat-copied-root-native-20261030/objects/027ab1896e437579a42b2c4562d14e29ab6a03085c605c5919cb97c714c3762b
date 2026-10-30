"""Real saved-data collection: preserve raw gates and canonical source bytes."""
import json,unittest
import packet as p

class CollectionControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):cls.data=p.historical.collect(exclude=('third/source-only-review/',))
    def test_only_duplicate_restoration_is_excluded(self):
        self.assertFalse(any(n.startswith('third/source-only-review/') for n in self.data))
        self.assertEqual(p.historical.source_files(self.data['first/input/source.tar.gz']).keys(),json.loads(self.data['first/input/source-map.json']).keys())
    def test_all_first_failed_and_successful_outcomes_are_byte_exact(self):
        for name in ('stage','metadata','check-B-194-adapter','check-O-194-adapter','lint-B-194-adapter','lint-O-194-adapter','test-B-194-adapter'):
            for file in ('result.json','execution.json','intent.json','stdout.log','stderr.log'):
                path=p.historical.THIRD/'native'/name/file
                self.assertEqual(self.data['third/native/'+name+'/'+file],path.read_bytes())
    def test_collected_result_never_admits_unexecuted_test(self):
        result=p.r.reconcile(self.data);self.assertTrue(result['stage_complete']);self.assertTrue(result['metadata_complete'])
        self.assertEqual(len(result['accepted_lanes']),4);self.assertEqual(result['first_refusal'],'test-B-194-adapter');self.assertFalse(result['full_focused_matrix_complete'])
        self.assertNotIn('third/native/test-B-194-adapter/cargo-test/intent.json',self.data)

if __name__=='__main__':unittest.main(verbosity=2)
