"""Rebuild exact inverse source edits from fixed producer commits, never HEAD."""
from pathlib import Path
import difflib
import hashlib
import json
import subprocess

ROOT = Path(__file__).resolve().parents[2]
REVIEWED = "3c52c0aa34b8d00b2fac44375d5ba8fdf532bb9f"
OLD = "c91068bfa8b297bb075b63457da235aaa9c6232e"
CURRENT = "958b3135a15c00b9f6aad41522cef9d1f6608d3a"
CASES = [
    ("docs/COMPAT.md", REVIEWED, "a67a5299ab335424e9b11330dc06bd749ea9b859"),
    ("docs/testing/gated-test-registry.toml", REVIEWED, CURRENT),
    *[("tools/long-run-supervisor-074/src/" + name, OLD, CURRENT) for name in (
        "diagnostic_authority_fixed.rs", "diagnostic_authority_fixture.rs",
        "diagnostic_kernel_origin.rs")],
]


def frozen(revision, path):
    return subprocess.check_output(
        ["git", "--no-replace-objects", "show", f"{revision}:{path}"], cwd=ROOT
    ).decode("utf-8").replace("\r\n", "\n")


def digest(raw):
    return hashlib.sha256(raw.encode("utf-8")).hexdigest()


def description():
    rows = []
    for path, old_rev, new_rev in CASES:
        old, new = frozen(old_rev, path), frozen(new_rev, path)
        before, after = old.splitlines(keepends=True), new.splitlines(keepends=True)
        patches = []
        for tag, i, end_i, j, end_j in difflib.SequenceMatcher(
            a=before, b=after, autojunk=False
        ).get_opcodes():
            if tag == "equal":
                continue
            patches.append(dict(
                offset=len("".join(after[:j]).encode("utf-8")),
                current="".join(after[j:end_j]), previous="".join(before[i:end_i]),
            ))
        assert patches and digest(new) != digest(old)
        rows.append(dict(path=path, previous_revision=old_rev, revision=new_rev,
                         current_sha256=digest(new), previous_sha256=digest(old),
                         patches=patches))
    return dict(schema="hc074-source-successor-view-v1", rows=rows)


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    data = (json.dumps(description(), ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    destination = Path(__file__).with_name("rules.json")
    if args.write:
        with destination.open("xb") as output:
            output.write(data)
    else:
        assert destination.read_bytes() == data, "fixed producer description differs"
    print(hashlib.sha256(data).hexdigest())
