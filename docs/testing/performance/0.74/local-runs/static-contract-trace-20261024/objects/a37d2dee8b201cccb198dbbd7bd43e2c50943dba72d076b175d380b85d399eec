"""Strict offline Trace2 review; overlapping Git spans are not target CPU."""
from collections import Counter
from decimal import Decimal
import json
import re


def require(condition, message):
    if not condition:
        raise ValueError(message)


def decode(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            require(key not in result, "duplicate Trace2 JSON key")
            result[key] = value
        return result

    def bad(value):
        raise ValueError(f"nonfinite Trace2 number: {value}")

    return json.loads(raw, object_pairs_hook=pairs, parse_float=Decimal, parse_constant=bad)


def ns(value):
    require(type(value) is int or isinstance(value, Decimal), "non-numeric Trace2 time")
    value = Decimal(value) * 1_000_000_000
    require(value.is_finite() and value >= 0 and value == value.to_integral_value(), "invalid Trace2 time")
    return int(value)


def session(raw):
    events = [decode(line) for line in raw.splitlines() if line]
    require(bool(events), "empty trace")
    require(events[0].get("event") == "version" and events[0].get("evt") == "4", "unsupported Trace2 version")
    sid = events[0].get("sid")
    require(isinstance(sid, str) and bool(sid), "missing Trace2 SID")
    require(all(event.get("sid") == sid for event in events), "mixed Trace2 sessions")
    require(not any(event.get("event") in {"too_many_files", "signal"} for event in events), "discarded or signalled trace")
    selected = {}
    positions = {}
    for kind in ["version", "start", "cmd_name", "exit", "atexit"]:
        matches = [(i, event) for i, event in enumerate(events) if event.get("event") == kind]
        require(len(matches) == 1, f"missing/duplicate Trace2 {kind}")
        positions[kind], selected[kind] = matches[0]
    require(list(positions.values()) == sorted(positions.values()), "Trace2 lifecycle order differs")
    require(events[-1].get("event") == "atexit", "trailing events after shutdown")
    argv = selected["start"].get("argv")
    require(isinstance(argv, list) and len(argv) >= 2 and all(isinstance(arg, str) for arg in argv), "invalid Git argv")
    require(argv[0].replace("\\", "/").rsplit("/", 1)[-1].lower() in {"git", "git.exe"}, "not Git")
    start, end, final = [ns(selected[kind].get("t_abs")) for kind in ["start", "exit", "atexit"]]
    require(start <= end <= final, "backwards Trace2 elapsed time")
    code = selected["exit"].get("code")
    require(type(code) is int and code == selected["atexit"].get("code"), "Git exit codes differ")
    name = selected["cmd_name"].get("name")
    require(isinstance(name, str) and bool(name), "missing Git command name")
    return dict(sid=sid, argv=argv, command=name, exit=code, elapsed_ns=final,
                after_start_ns=final-start, nested="/" in sid)


def immutable_shaped_key(argv):
    args = argv[1:]
    if args[:1] == ["--no-replace-objects"]:
        args = args[1:]
    if len(args) != 2 or args[0] != "show":
        return None
    revision, separator, path = args[1].partition(":")
    if not separator or not re.fullmatch(r"[0-9a-f]{40}", revision):
        return None
    if not path or any(part in {"", ".", ".."} for part in path.split("/")):
        return None
    if not re.fullmatch(r"[A-Za-z0-9_./-]+", path):
        return None
    return args[1]


def summarize(traces):
    require(bool(traces), "no Git trace sessions")
    sessions = [session(raw) for raw in traces]
    require(len({item["sid"] for item in sessions}) == len(sessions), "duplicate Git session")
    roots = [item for item in sessions if not item["nested"]]
    keys = Counter(key for item in roots if (key := immutable_shaped_key(item["argv"])) is not None)
    return dict(
        sessions=len(sessions), root_sessions=len(roots), nested_sessions=len(sessions)-len(roots),
        commands=dict(sorted(Counter(item["command"] for item in sessions).items())),
        nonzero_exits=dict(sorted(Counter(str(item["exit"]) for item in sessions if item["exit"]).items())),
        overlapping_git_elapsed_ns=sum(item["elapsed_ns"] for item in sessions),
        full_hash_show_requests=sum(keys.values()), unique_full_hash_show_keys=len(keys),
        repeated_full_hash_show_requests=sum(count-1 for count in keys.values()),
        repeated_keys=[dict(key=key, requests=count) for key, count in sorted(keys.items(), key=lambda pair:(-pair[1],pair[0])) if count > 1],
        cache_admission=False, cpu_attribution=False, per_test_attribution=False,
        spawn_overhead_included=False, overlapping_times_additive_to_target=False,
    )


def listed_cases(raw):
    lines = raw.splitlines()
    names = [line[:-6] for line in lines if line.endswith(": test")]
    require(len(names) == len(set(names)), "duplicate listed test")
    totals = [re.fullmatch(r"(\d+) tests, (\d+) benchmarks", line) for line in lines]
    totals = [match for match in totals if match]
    require(len(totals) == 1 and int(totals[0][1]) == len(names) and int(totals[0][2]) == 0, "listed test counts differ")
    return sorted(names)


def case_result(raw, expected):
    starts = re.findall(r"^running (\d+) tests$", raw, re.M)
    cases = re.findall(r"^test (\S+) \.\.\. (ok|FAILED|ignored)$", raw, re.M)
    summaries = re.findall(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out; finished in (\d+(?:\.\d+)?)s$", raw, re.M)
    require(starts == [str(len(expected))] and len(summaries) == 1, "missing or partial target result")
    require(sorted(name for name, _ in cases) == sorted(expected), "observed case catalog differs")
    summary = summaries[0]
    require(summary[0] == "ok" and list(map(int, summary[1:6])) == [len(expected), 0, 0, 0, 0], "target refused or scope changed")
    require(all(verdict == "ok" for _, verdict in cases), "nonpassing case verdict")
    return dict(passed=len(cases), failed=0, ignored=0, filtered=0, reported_seconds=summary[6])
