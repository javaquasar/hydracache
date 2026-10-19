"""Preregistered paired wall ratios, not authorization or product performance."""
import argparse
import json
import math
import random
import re
from pathlib import Path


def evaluate(rows):
    result = {}
    for leaf in ("boot", "namespace"):
        selected = [r for r in rows if r["leaf"] == leaf]
        if len(selected) != 45 or any(r["iterations"] != 16384 for r in selected):
            raise ValueError("require three complete 15-block repetitions per leaf")
        ratios = [math.log(r["b_ns"] / r["a_ns"]) for r in selected]
        if not all(math.isfinite(x) for x in ratios):
            raise ValueError("nonfinite paired ratio")
        rng = random.Random(0x0740F17E)
        samples = sorted(
            100 * (1 - math.exp(sum(rng.choices(ratios, k=45)) / 45))
            for _ in range(5000)
        )
        result[leaf] = dict(
            blocks=45,
            improvement_percent=100 * (1 - math.exp(sum(ratios) / 45)),
            low95=samples[124],
            high95=samples[4874],
        )
    result["accepted"] = (
        all(result[k]["improvement_percent"] > 0 for k in ("boot", "namespace"))
        and any(result[k]["low95"] > 0 for k in ("boot", "namespace"))
        and all(result[k]["high95"] >= 0 for k in ("boot", "namespace"))
    )
    return result


def selftest():
    def rows(b):
        return [dict(leaf=k, iterations=16384, a_ns=100, b_ns=b)
                for k in ("boot", "namespace") for _ in range(45)]
    assert evaluate(rows(90))["accepted"]
    assert not evaluate(rows(100))["accepted"]
    assert not evaluate(rows(110))["accepted"]
    for bad in (rows(90)[:-1], rows(0)):
        try:
            evaluate(bad)
        except (ValueError, ZeroDivisionError):
            pass
        else:
            raise AssertionError("malformed observations accepted")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("directory", type=Path)
    parser.add_argument("--prefix", default="")
    args = parser.parse_args()
    selftest()
    results = {}
    for compiler in ("gnu194", "gnu188"):
        rows = []
        for repeat in range(1, 4):
            raw = (args.directory / f"{args.prefix}{compiler}" / f"leaf-{repeat}.log").read_text(encoding="utf-8-sig")
            if "test result: ok. 1 passed; 0 failed;" not in raw:
                raise ValueError("incomplete/zero test selection")
            part = [json.loads(x) for x in re.findall(r"HC074-LEAF-PATH (\{[^\r\n]+\})", raw)]
            if len(part) != 30:
                raise ValueError("wrong block count")
            order = "ABBA-BAAB" if args.prefix == "aa3-" else "external-v3"
            if args.prefix != "aa2-" and any(r["order"] != order for r in part):
                raise ValueError("require balanced indirect-call harness")
            for leaf in ("boot", "namespace"):
                if sorted(r["block"] for r in part if r["leaf"] == leaf) != list(range(15)):
                    raise ValueError("missing or duplicate paired block")
            rows.extend(part)
        results[compiler] = evaluate(rows)
    results["accepted_both"] = all(results[k]["accepted"] for k in ("gnu194", "gnu188"))
    results["seed"] = "0x0740f17e"
    results["resamples"] = 5000
    print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
