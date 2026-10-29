"""Synthetic controls; no target, manager or product workload is run."""
import json
import unittest

import trace_review as review


def trace(sid="test", argv=None, code=0):
    base = dict(sid=sid, thread="main")
    events = [
        dict(base,event="version",evt="4",exe="2.51.0.windows.1"),
        dict(base,event="start",t_abs=0.001,argv=argv or ["git","show","a"*40+":file.rs"]),
        dict(base,event="cmd_name",name="show"),
        dict(base,event="exit",t_abs=0.002,code=code),
        dict(base,event="atexit",t_abs=0.003,code=code),
    ]
    return "\n".join(json.dumps(event) for event in events)


class TraceTests(unittest.TestCase):
    def test_elapsed_is_per_process_and_repeats_are_not_admission(self):
        value = review.summarize([trace("one"),trace("two"),trace("one/child")])
        self.assertEqual(value["sessions"],3)
        self.assertEqual(value["overlapping_git_elapsed_ns"],9_000_000)
        self.assertEqual(value["full_hash_show_requests"],2)
        self.assertEqual(value["repeated_full_hash_show_requests"],1)
        self.assertFalse(value["cache_admission"])
        self.assertFalse(value["spawn_overhead_included"])

    def test_nonzero_git_is_retained_not_changed_to_success(self):
        value = review.summarize([trace(code=128)])
        self.assertEqual(value["nonzero_exits"],{"128":1})

    def test_duplicate_session_empty_or_truncated_refuses(self):
        for samples in [[],[trace(),trace()],[trace().rsplit("\n",1)[0]]]:
            with self.subTest(samples=samples),self.assertRaises(ValueError):
                review.summarize(samples)

    def test_mutated_lifecycle_format_code_time_or_sid_refuses(self):
        original = trace().splitlines()
        for index,field,value in [(0,"evt","5"),(1,"sid","other"),(3,"code",1),(3,"t_abs",0),(4,"event","signal")]:
            events = [json.loads(line) for line in original]
            events[index][field] = value
            with self.subTest(field=field),self.assertRaises(ValueError):
                review.session("\n".join(json.dumps(event) for event in events))
        with self.assertRaises(ValueError):
            review.session("\n".join(original+[original[-1]]))
        with self.assertRaises(ValueError):
            review.session("\n".join([original[0],original[2],original[1],*original[3:]]))

    def test_duplicate_json_and_nonfinite_refuse(self):
        for raw in ['{"a":1,"a":2}','{"t_abs":NaN}','{"t_abs":Infinity}']:
            with self.assertRaises(ValueError):
                review.decode(raw)

    def test_only_exact_full_hash_show_shape_is_counted(self):
        good = "a"*40+":path/file.rs"
        self.assertEqual(review.immutable_shaped_key(["git","show",good]),good)
        self.assertEqual(review.immutable_shaped_key(["git","--no-replace-objects","show",good]),good)
        for args in [["git","show","HEAD:file"],["git","show","abcd:file"],
                     ["git","show","a"*40+":../file"],["git","show",good,"extra"],
                     ["git","ls-tree","a"*40]]:
            self.assertIsNone(review.immutable_shaped_key(args))

    def test_case_listing_and_complete_result_are_exact(self):
        expected = review.listed_cases("one: test\ntwo: test\n\n2 tests, 0 benchmarks\n")
        raw = ("running 2 tests\ntest two ... ok\ntest one ... ok\n"
               "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.00s\n")
        self.assertEqual(review.case_result(raw,expected)["passed"],2)
        for bad in [raw.replace("test one ... ok\n",""),raw.replace("test one","test two"),
                    raw.replace("0 filtered","1 filtered"),raw.replace("one ... ok","one ... FAILED")]:
            with self.assertRaises(ValueError):
                review.case_result(bad,expected)
        with self.assertRaises(ValueError):
            review.listed_cases("one: test\none: test\n2 tests, 0 benchmarks\n")


if __name__ == "__main__":
    unittest.main()
