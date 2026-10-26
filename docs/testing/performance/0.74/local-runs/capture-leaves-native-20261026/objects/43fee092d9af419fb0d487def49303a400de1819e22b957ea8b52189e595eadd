"""Data-only replay of the sealed prior 321-case target; no current gate credit."""

import json
from pathlib import Path
import sys

import review
from target_result_join import ExpectedSource, catalog_for, join_target


def analyze(root):
    folder = root / "docs/testing/performance/0.74/local-runs/repaired-review-frontier-20261025"
    raw = (folder / "index.json").read_bytes()
    review.require(review.sha(raw) == "2bc4c942e01c9b9dec15c14046afeb1b5c71e022b76d89d4ea07f23d89d1e012", "historical packet drift")
    index = review.decode_json(raw)
    data = {name: review.member(folder, index, "selected-target/" + name)
            for name in ("intent.json", "result.json", "stdout.log", "stderr.log")}
    expected = ExpectedSource("050d0b3fe2592fcce414c18b3aa21cd91a64e5a5",
                              "e8a60c7cef23390665a4ad3dd40fde832966bdbb23f922762bf6ee7fc4a97e6c")
    result = join_target(review.decode_json(data["intent.json"]), review.decode_json(data["result.json"]),
                         data["stdout.log"].decode(), data["stderr.log"].decode(), expected, catalog_for(root))
    archived_pass = result.pop("selected_target_pass")
    result.update(archived_selected_target_pass=archived_pass, historical_replay_only=True,
                  new_target_executed=False, live_attempt_proven=False, current_release_gate_pass=False)
    return result


def main(output):
    out = Path(output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    (out / "intent.json").write_text(json.dumps(dict(historical_replay_only=True, producer_repeated=False)), encoding="utf-8")
    try:
        result = analyze(Path(__file__).resolve().parents[2])
    except Exception as error:
        result = dict(first_refusal=f"{type(error).__name__}: {error}", current_release_gate_pass=False, admission=False)
    (out / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return "first_refusal" not in result


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] != "replay":
        raise SystemExit("usage: historical_target_replay.py replay NEW_OUTPUT_DIR")
    raise SystemExit(0 if main(sys.argv[2]) else 1)
