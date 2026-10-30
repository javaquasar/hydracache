import unittest
import graph as p
import common as c

class ExactHistory(unittest.TestCase):
    def test_exact_two_node_direct_edge(self):
        old=p.historical_lock()
        new=p.candidate(old)
        self.assertEqual(p.review(old,new)['candidate_nodes'],685)
        # Every unapproved byte change, including comments and unrelated nodes, refuses.
        for mutated in (new+b'\n',new.replace(b'"zstd 0.13.3"',b'"zstd 0.14.3"'),
                        new.replace(b'version = "0.73.0"',b'version = "0.74.0"',1),
                        new.replace(b'"zstd-safe 7.2.4"',b'"zstd-safe 8.2.0"')):
            with self.assertRaises(ValueError):p.review(old,mutated)
        with self.assertRaises(ValueError):p.candidate(old+b'\n')

if __name__=='__main__':unittest.main()
