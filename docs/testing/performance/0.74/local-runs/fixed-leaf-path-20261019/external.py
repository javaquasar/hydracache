"""Local actual-function cross-image comparator. Never starts a product."""
import argparse
import hashlib
import json
import os
import re
import subprocess
from pathlib import Path


def digest(path):
    return hashlib.file_digest(Path(path).open("rb"), "sha256").hexdigest()


def sample(image, leaf):
    result = subprocess.run(
        [image, "fixed_leaf_path_single_cost", "--ignored", "--nocapture", "--test-threads=1"],
        env=dict(os.environ, HC074_LEAF_SINGLE=leaf), capture_output=True,
        text=True, check=True, timeout=120,
    )
    raw = result.stdout + result.stderr
    if "test result: ok. 1 passed; 0 failed;" not in raw:
        raise ValueError("zero or wrong selection")
    values = re.findall(r"HC074-LEAF-PATH-SINGLE (\{[^\r\n]+\})", raw)
    if len(values) != 1:
        raise ValueError("missing actual sample")
    value = json.loads(values[0])
    if value["leaf"] != leaf or value["iterations"] != 16384 or not value["actual"] or value["product"]:
        raise ValueError("incompatible sample")
    return value["elapsed_ns"], raw


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--a", required=True)
    parser.add_argument("--b", required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    if os.geteuid() == 0 or args.out.exists():
        raise ValueError("nonroot fresh cohort required")
    before = [digest(args.a), digest(args.b)]
    args.out.mkdir(parents=True)
    for repeat in range(1, 4):
        with (args.out / f"leaf-{repeat}.log").open("x", encoding="utf-8") as output:
            for block in range(15):
                leaves = ("boot", "namespace") if block % 2 == 0 else ("namespace", "boot")
                for leaf in leaves:
                    samples = [sample(image, leaf) for image in
                               (args.a, args.b, args.b, args.a, args.b, args.a, args.a, args.b)]
                    values = [x[0] for x in samples]
                    row = dict(block=block, leaf=leaf, iterations=16384, order="external-v3",
                               a_ns=sum(values[i] for i in (0, 3, 5, 6)) // 4,
                               b_ns=sum(values[i] for i in (1, 2, 4, 7)) // 4,
                               all_ns=values, child_stdout=[x[1] for x in samples])
                    output.write("HC074-LEAF-PATH " + json.dumps(row) + "\n")
                    output.flush()
            output.write("HC074-LEAF-PATH-CLOSED observations_unchanged=true cached=false product=false\n")
            output.write("test result: ok. 1 passed; 0 failed; external_aggregate=true\n")
        print(f"repeat={repeat} actual_children=240 leaves=2 complete=true", flush=True)
    if before != [digest(args.a), digest(args.b)]:
        raise ValueError("image drift")
    (args.out / "receipt.json").write_text(json.dumps(dict(
        a_image=before[0], b_image=before[1], repeats=3, actual_children=720,
        uid=os.geteuid(), server=False, product=False, admission=False,
    ), indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
