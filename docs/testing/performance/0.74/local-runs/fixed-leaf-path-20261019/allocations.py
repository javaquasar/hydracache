"""Verify the preregistered Rust-request endpoint, never a timing claim."""
import copy
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parent
FIELDS = ("allocations", "reallocations", "requested_bytes")


def rows(label):
    raw = (ROOT / label / "allocation.log").read_text(encoding="utf-8-sig")
    assert "test result: ok. 1 passed; 0 failed;" in raw
    assert "HC074-LEAF-ALLOC-CLOSED timer_changed=false server=false product=false" in raw
    result = [json.loads(m) for m in re.findall(r"HC074-LEAF-ALLOC (\{[^\r\n]+\})", raw)]
    assert len(result) == 6
    assert {(r["sample"], r["leaf"]) for r in result} == {
        (i, leaf) for i in range(3) for leaf in ("boot", "namespace")
    }
    for row in result:
        assert row["iterations"] == 1024 and row["rust_requests_only"] is True
        for side in ("reference", "candidate"):
            assert set(row[side]) == set(FIELDS)
            assert all(type(row[side][k]) is int and row[side][k] >= 0 for k in FIELDS)
    return sorted(result, key=lambda r: (r["leaf"], r["sample"]))


def compare(baseline, candidate):
    assert len(baseline) == len(candidate) == 6
    result = []
    for old, new in zip(baseline, candidate):
        assert (old["sample"], old["leaf"]) == (new["sample"], new["leaf"])
        a, b = new["reference"], new["candidate"]
        assert old["reference"] == old["candidate"] == a, "A/A or reference drift"
        assert all(b[k] <= a[k] for k in FIELDS), "cost increased"
        assert b["allocations"] < a["allocations"] or b["reallocations"] < a["reallocations"]
        result.append({"sample": new["sample"], "leaf": new["leaf"],
                       "iterations": 1024, "before": a, "after": b})
    return result


def analyze():
    return {"schema": 1, "endpoint": "rust-allocation-requests-only",
            "allocation_rule_passed": True, "wall_speedup_claimed": False,
            "server": False, "product": False,
            "compilers": {c: compare(rows("alloc-baseline-gnu" + c),
                                     rows("alloc-candidate-gnu" + c + ("-retry" if c == "194" else "")))
                          for c in ("194", "188")}}


def self_test():
    row = {"sample": 0, "leaf": "boot", "reference": dict(zip(FIELDS, (5, 0, 62))),
           "candidate": dict(zip(FIELDS, (5, 0, 62)))}
    baseline = [copy.deepcopy(row) for _ in range(6)]
    candidate = copy.deepcopy(baseline)
    for r in candidate:
        r["candidate"] = dict(zip(FIELDS, (1, 0, 36)))
    assert len(compare(baseline, candidate)) == 6
    for fault in ("aa", "reference", "unchanged", "bytes", "realloc", "missing", "order"):
        a, b = copy.deepcopy(baseline), copy.deepcopy(candidate)
        if fault == "aa":
            a[0]["candidate"]["allocations"] += 1
        elif fault == "reference":
            b[0]["reference"]["allocations"] += 1
        elif fault == "unchanged":
            b[0]["candidate"] = copy.deepcopy(b[0]["reference"])
        elif fault == "bytes":
            b[0]["candidate"]["requested_bytes"] = 63
        elif fault == "realloc":
            b[0]["candidate"]["reallocations"] = 1
        elif fault == "missing":
            b.pop()
        else:
            b[0]["sample"] = 1
        try:
            compare(a, b)
        except AssertionError:
            continue
        raise AssertionError("accepted mutation: " + fault)
    print("allocation analysis mutation controls: 7 passed")


if __name__ == "__main__":
    if sys.argv[1:] == ["--self-test"]:
        self_test()
    elif not sys.argv[1:]:
        print(json.dumps(analyze(), indent=2))
    else:
        raise SystemExit("usage: allocations.py [--self-test]")
