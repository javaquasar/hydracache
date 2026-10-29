import unittest
import getunit_seal as s

class ClockControls(unittest.TestCase):
    def event(self,clock): return {'site':'clock-order','clock':clock}
    def test_exact_real_time_regression(self):
        result=s.clock_reason(self.event([100,200,500,400,1,60_000_000_000]))
        self.assertEqual(result['reason'],'realtime_backwards')
        self.assertEqual(result['realtime_delta_ns'],-100)
        self.assertEqual(result['boottime_delta_ns'],100)
        self.assertIsNone(result['caused_by_specific_service'])
    def test_original_condition_order(self):
        self.assertEqual(s.clock_reason(self.event([200,100,500,400,0,60_000_000_000]))['reason'],'boot_changed')
        self.assertEqual(s.clock_reason(self.event([200,100,500,400,1,60_000_000_000]))['reason'],'boottime_backwards')
    def test_healthy_order_cannot_be_labelled_refusal(self):
        with self.assertRaises(ValueError): s.clock_reason(self.event([100,200,400,500,1,60_000_000_000]))
    def test_unknown_contract_or_shape_refuses(self):
        for clock in ([1,2],[1,2,3,2,2,60_000_000_000],[1,2,3,2,1,90_000_000_000]):
            with self.assertRaises(ValueError): s.clock_reason(self.event(clock))

if __name__=='__main__': unittest.main()
