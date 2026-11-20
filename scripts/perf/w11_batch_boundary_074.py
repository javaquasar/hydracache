"""Preserve batch boundary observations for a separately admitted W11 build.

The caller owns the isolated daemon, the verified build subject and the
effective config and process observations. This collector cannot start a
daemon or authorize a build, and it never publishes a qualification result.
"""

from dataclasses import dataclass
from hashlib import sha256
import os
import stat

from . import performance_signed_role_probe_074 as probe
from . import w11_product_build_receipt_074 as receipt


OK = b"+OK\r\n"
TOO_LARGE = b"-ERR HydraCache client error: request too large\r\n"
CHECK_CHUNK = 128
DEFAULT = "default"
AUTH_FILES = ("no-certificate.resp", "denied.resp", "authenticated.resp", "fresh-connection.resp")


@dataclass(frozen=True)
class Step:
    name: str
    request: bytes
    reply: bytes


class RespSession:
    def __init__(self, material, address, token, deadline, expected_version):
        probe.require(address[0] == "127.0.0.1" and 0 < address[1] <= 65535,
                      "the batch proof requires the admitted loopback listener")
        probe.require(0 < probe.seconds_left(deadline) <= probe.PROCESS_LIMIT,
                      "batch proof must use the original process deadline")
        self.material = material
        self.address = address
        self.token = token
        self.deadline = deadline
        self.version = expected_version

    def verify_authentication(self, output):
        probe.protocol(self.material, self.address, self.token, self.deadline, output,
                       expected_version=self.version)

    def issue(self, command):
        hello = probe.wire(b"HELLO", b"3", b"AUTH", b"default", self.token)
        reply = probe.exchange(self.material, self.address,
                               hello + command + probe.wire(b"QUIT"), self.deadline)
        tail = probe.split_resp3_hello(reply, expected_version=self.version)
        probe.require(tail.endswith(OK), "missing QUIT reply on a new authenticated connection")
        payload = tail[:-len(OK)]
        probe.require(self.token not in payload, "batch reply contains the fixture credential")
        return payload


def case_parameters(selector):
    probe.require(selector in (None, receipt.SELECTOR), "unsupported batch proof profile")
    accepted, refused = ((128,), (129,)) if selector is None else (receipt.ACCEPTED, receipt.REFUSED)
    return [(command, count, outcome)
            for command in receipt.COMMANDS
            for counts, outcome in ((accepted, "accepted"), (refused, "refused"))
            for count in counts]


def array_reply(values):
    return (b"*%d\r\n" % len(values)) + b"".join(
        b"_\r\n" if item is None else (b"$%d\r\n%s\r\n" % (len(item), item))
        for item in values
    )


def mget(name, keys, values):
    return Step(name, probe.wire(b"MGET", *keys), array_reply([values.get(key) for key in keys]))


def mset(name, entries, expected):
    return Step(name, probe.wire(b"MSET", *(part for pair in entries for part in pair)), expected)


def content(tag, index, count):
    return b"\0\xff\r\n" + tag + index.to_bytes(8, "big") + count.to_bytes(2, "big")


def check_state(label, keys, values):
    return [
        mget(f"{label}-{offset // CHECK_CHUNK}",
             keys[offset:offset + CHECK_CHUNK], values)
        for offset in range(0, len(keys), CHECK_CHUNK)
    ]


def case_steps(selector, command, count, outcome):
    probe.require((command, count, outcome) in case_parameters(selector), "unregistered batch case")
    label = ("d" if selector is None else "w") + ("g" if command == "MGET" else "s")
    keys = tuple(f"w11{label}{count:03d}{index:08d}".encode("ascii") for index in range(count))
    initial = {key: content(b"v0", n, count) for n, key in enumerate(keys) if n % 2}
    steps = [mset("seed", initial.items(), OK), *check_state("before", keys, initial)]
    after = initial
    if command == "MGET":
        steps.append(mget("boundary", keys[::-1], initial) if outcome == "accepted" else
                     Step("boundary", probe.wire(b"MGET", *keys[::-1]), TOO_LARGE))
        steps.append(mget("duplicates", (keys[1], keys[0], keys[1], keys[0]), initial))
    else:
        changed = {key: content(b"v1", n, count) for n, key in enumerate(keys)}
        steps.append(mset("boundary", changed.items(), OK if outcome == "accepted" else TOO_LARGE))
        if outcome == "accepted":
            after = changed
    return steps + check_state("after", keys, after)


def write_bytes(path, data):
    free = probe.disk_guard(path.parent)
    block = os.statvfs(path.parent).f_frsize if hasattr(os, "statvfs") else 4096
    allocated = (len(data) + block - 1) // block * block
    probe.require(free - probe.DISK_FLOOR >= allocated, "insufficient space for batch proof")
    with path.open("xb") as stream:
        stream.write(data)
        stream.flush()
        os.fsync(stream.fileno())
    probe.disk_guard(path.parent)


def read_capture(path, limit):
    info = path.lstat()
    probe.require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_size <= limit,
                  "unexpected batch proof file or length")
    with path.open("rb") as stream:
        probe.require(os.path.samestat(info, os.fstat(stream.fileno())), "batch proof file changed")
        data = stream.read(limit + 1)
    probe.require(len(data) == info.st_size, "batch proof size changed")
    return data


def auth_index(directory):
    probe.require({path.name for path in directory.iterdir()} == set(AUTH_FILES),
                  "incomplete authentication proof")
    return {name: sha256(read_capture(directory / name, 4096)).hexdigest() for name in AUTH_FILES}


def observed_step(step, request, reply):
    probe.require(request == step.request and reply == step.reply,
                  f"batch case {step.name} differs from its byte oracle")
    return {"step": step.name, "request_sha256": sha256(request).hexdigest(),
            "response_sha256": sha256(reply).hexdigest()}


def oracle_body(rows, auth):
    return receipt.canonical({"schema": "w11-batch-semantic-oracle-074-v1",
                              "authentication": auth, "steps": rows}) + b"\n"


def case_row(command, count, outcome, rows, text):
    result = next(row for row in rows if row["step"] == "boundary")
    return {"command": command, "entries": count, "outcome": outcome,
            "request_sha256": result["request_sha256"], "response_sha256": result["response_sha256"],
            "oracle_sha256": sha256(text).hexdigest()}


def collect_case(selector, command, count, outcome, session, directory, auth):
    directory.mkdir(mode=0o700)
    rows = []
    for step in case_steps(selector, command, count, outcome):
        write_bytes(directory / f"{step.name}.request.resp", step.request)
        reply = session.issue(step.request)
        write_bytes(directory / f"{step.name}.response.resp", reply)
        rows.append(observed_step(step, step.request, reply))
    oracle = oracle_body(rows, auth)
    write_bytes(directory / "oracle.json", oracle)
    return case_row(command, count, outcome, rows, oracle)


def collect_profile(subject, selector, effective_config, observe, session, directory):
    probe.require(receipt.canonical(effective_config) == receipt.canonical(receipt.config(selector)),
                  "effective batch config differs before the first command")
    cases = case_parameters(selector)
    probe.require(subject["schema"] == receipt.RECEIPT_SCHEMA and subject["role"] in receipt.build.ROLES,
                  "expected the independently admitted v2 build subject")
    image = subject["binary"]["sha256"]
    before = dict(observe())
    generation = receipt.generation(before, image)
    probe.disk_guard(directory.parent)
    directory.mkdir(mode=0o700)
    auth_dir = directory / "auth"
    auth_dir.mkdir(mode=0o700)
    session.verify_authentication(auth_dir)
    auth = auth_index(auth_dir)
    rows = []
    for command, count, outcome in cases:
        probe.seconds_left(session.deadline)
        probe.require(receipt.generation(observe(), image) == generation, "daemon generation changed")
        rows.append(collect_case(selector, command, count, outcome, session,
                                 directory / f"{command}-{count}", auth))
    after = dict(observe())
    probe.seconds_left(session.deadline)
    probe.require(receipt.generation(after, image) == generation, "daemon generation changed")
    return {"effective_config": effective_config.copy(), "before": before, "after": after, "cases": rows}


def read_case_archive(selector, command, count, outcome, directory, auth):
    rows = []
    for step in case_steps(selector, command, count, outcome):
        request = read_capture(directory / f"{step.name}.request.resp", len(step.request))
        reply = read_capture(directory / f"{step.name}.response.resp", len(step.reply))
        rows.append(observed_step(step, request, reply))
    expected = oracle_body(rows, auth)
    oracle = read_capture(directory / "oracle.json", len(expected))
    probe.require(oracle == expected, "batch oracle changed after capture")
    return case_row(command, count, outcome, rows, oracle)


def publish_evidence(subject, default, opt_in, archives, artifact):
    image = subject["binary"]["sha256"]
    generations = []
    for label, selector, result, accepted, refused in (
        (DEFAULT, None, default, (128,), (129,)),
        (receipt.SELECTOR, receipt.SELECTOR, opt_in, receipt.ACCEPTED, receipt.REFUSED),
    ):
        generations.append(receipt.validate_run(result, selector, image, accepted, refused))
        auth = auth_index(archives / label / "auth")
        verified = [read_case_archive(selector, command, count, outcome,
                                      archives / label / f"{command}-{count}", auth)
                    for command, count, outcome in case_parameters(selector)]
        probe.require(verified == result["cases"], "issued case digest differs from the retained bytes")
    probe.require(generations[0] != generations[1], "profile observations reuse a daemon generation")
    document = {"schema": receipt.EVIDENCE_SCHEMA, "build": subject,
                DEFAULT: default, receipt.SELECTOR: opt_in}
    body = receipt.canonical(document) + b"\n"
    probe.require(len(body) <= receipt.MAX_JSON_BYTES, "batch proof exceeds its receipt limit")
    write_bytes(artifact / receipt.EVIDENCE, body)
    return sha256(body).hexdigest()
