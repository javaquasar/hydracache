"""Transfer one exact build and its raw observations across a no-NIC guest."""

from base64 import b64decode, b64encode
from hashlib import sha256
from io import BytesIO
import os
from pathlib import Path
import secrets
import stat
import tempfile
from zipfile import ZIP_STORED, ZipFile, ZipInfo

from . import w11_build_workflow_074 as workflow


REQUEST = "guest-request.json"
SCHEMA = "hydracache-w11-guest-transfer-074-v1"
PREFIX = b"W11V2 "
CHUNK = 3072
MAX_RESULT = 2 * 1024**2
MAX_SERIAL = 3 * 1024**2
BUILD_CONTEXT_KEYS = (
    "GITHUB_REPOSITORY", "GITHUB_REPOSITORY_ID", "GITHUB_REF", "GITHUB_WORKFLOW_REF",
    "GITHUB_SHA", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT",
)
receipt = workflow.receipt
build = workflow.build
batch = workflow.batch


def request_bytes(source, incoming, binding, scope):
    build.require(scope in {"fixture", "protected"}, "unexpected guest request scope")
    subject = workflow.check_build(source, incoming, binding)
    return receipt.canonical({"schema": SCHEMA, "scope": scope,
                              "nonce": secrets.token_hex(32), "subject": subject}) + b"\n"


def check_request(raw, expected_digest, source, incoming, binding, scope):
    build.require(0 < len(raw) <= receipt.MAX_JSON_BYTES
                  and sha256(raw).hexdigest() == expected_digest, "guest request SHA-256 differs")
    value = build.decode_json(raw)
    build.require(isinstance(value, dict) and set(value) == {"schema", "scope", "nonce", "subject"}
                  and value["schema"] == SCHEMA and value["scope"] == scope
                  and scope in {"fixture", "protected"} and receipt.is_hex(value["nonce"], 64)
                  and raw == receipt.canonical(value) + b"\n", "invalid or unauthorized guest request")
    subject = workflow.check_build(source, incoming, binding)
    build.require(value["subject"] == subject, "guest input belongs to another build or attempt")
    return subject


def copy_input(raw, expected_digest, source, incoming, binding, scope, destination):
    subject = check_request(raw, expected_digest, source, incoming, binding, scope)
    build.require(not destination.exists(), "guest input destination already exists")
    sizes = {workflow.SUBJECT: len(receipt.canonical(subject)) + 1, subject["binary"]["name"]: subject["binary"]["bytes"]}
    with tempfile.TemporaryDirectory(prefix="w11-guest-input-", dir=destination.parent) as scratch:
        shelf = Path(scratch) / "verified"
        shelf.mkdir(mode=0o700)
        for name, size in sizes.items():
            path = incoming / name
            info = path.lstat()
            build.require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_size == size,
                          "guest input file metadata differs")
            with path.open("rb") as read, (shelf / name).open("xb") as write:
                build.require(os.path.samestat(info, os.fstat(read.fileno())), "guest input changed before copy")
                remaining = size
                while remaining:
                    data = read.read(min(remaining, 1024**2))
                    build.require(bool(data), "guest input shortened during copy")
                    write.write(data)
                    remaining -= len(data)
                build.require(not read.read(1), "guest input grew during copy")
        build.require(workflow.check_build(source, shelf, binding) == subject, "guest copy differs")
        shelf.rename(destination)
    return subject


def leaf_inventory():
    names = {receipt.EVIDENCE, f"archives/{workflow.SUBJECT}"}
    for label, selector in ((batch.DEFAULT, None), (receipt.SELECTOR, receipt.SELECTOR)):
        folder = f"archives/{label}"
        names.update(f"{folder}/{leaf}" for leaf in ("effective-config.json", "ready.json", "listener.json", "run.json"))
        names.update(f"{folder}/auth/{leaf}" for leaf in batch.AUTH_FILES)
        for command, count, outcome in batch.case_parameters(selector):
            root = f"{folder}/{command}-{count}"
            names.add(f"{root}/oracle.json")
            for step in batch.case_steps(selector, command, count, outcome):
                names.update(f"{root}/{step.name}.{suffix}.resp" for suffix in ("request", "response"))
    return names


def leaf_limit(name):
    return 16 * 1024 if name.endswith(".resp") else receipt.MAX_JSON_BYTES


def verify_observations(folder, subject):
    workflow.check_names(folder, ["archives", receipt.EVIDENCE])
    evidence = workflow.read_canonical(folder / receipt.EVIDENCE)
    return workflow.verify_evidence_archive(folder / "archives", evidence, subject)


def result_bytes(folder, subject):
    verify_observations(folder, subject)
    output = BytesIO()
    with ZipFile(output, "w", compression=ZIP_STORED, allowZip64=False) as packet:
        for name in sorted(leaf_inventory()):
            data = batch.read_capture(folder / name, leaf_limit(name))
            member = ZipInfo(name)
            member.create_system = 3
            member.external_attr = 0o100600 << 16
            packet.writestr(member, data)
            build.require(output.tell() <= MAX_RESULT, "guest result exceeds the transport bound")
    body = output.getvalue()
    build.require(0 < len(body) <= MAX_RESULT, "guest result exceeds the transport bound")
    return body


def frame_response(request, body):
    build.require(0 < len(body) <= MAX_RESULT, "guest result exceeds the transport bound")
    context = sha256(request).hexdigest().encode("ascii")
    digest = sha256(body).hexdigest().encode("ascii")
    rows = [PREFIX + b"BEGIN " + context + b" " + str(len(body)).encode("ascii") + b" " + digest]
    for start in range(0, len(body), CHUNK):
        rows.append(PREFIX + b"DATA " + b64encode(body[start:start + CHUNK]))
    rows.append(PREFIX + b"END " + context + b" " + digest)
    return b"\n".join(rows) + b"\n"


def read_response(request, serial):
    build.require(len(serial) <= MAX_SERIAL, "guest response channel exceeded its bound")
    rows = [row[len(PREFIX):] for row in serial.splitlines() if row.startswith(PREFIX)]
    build.require(len(rows) >= 3, "incomplete guest response")
    header = rows[0].split(b" ")
    context = sha256(request).hexdigest().encode("ascii")
    build.require(len(header) == 4 and header[0] == b"BEGIN" and header[1] == context
                  and header[2].isdigit() and len(header[2]) <= 7
                  and receipt.is_hex(header[3].decode("ascii"), 64), "wrong guest response context")
    length = int(header[2])
    build.require(0 < length <= MAX_RESULT and rows[-1] == b"END " + context + b" " + header[3],
                  "wrong guest response end or size")
    body = bytearray()
    for row in rows[1:-1]:
        build.require(row.startswith(b"DATA ") and 5 < len(row) <= 5 + 4 * CHUNK // 3,
                      "unexpected or repeated guest response frame")
        piece = b64decode(row[5:], validate=True)
        build.require(len(piece) == min(CHUNK, length - len(body)) > 0
                      and b64encode(piece) == row[5:], "guest response length or encoding differs")
        body.extend(piece)
    build.require(len(body) == length and sha256(body).hexdigest().encode("ascii") == header[3],
                  "guest response digest differs")
    return bytes(body)


def receive_result(request, serial, subject, destination):
    build.require(not destination.exists(), "guest result destination already exists")
    body = read_response(request, serial)
    wanted = leaf_inventory()
    with ZipFile(BytesIO(body)) as packet:
        members = packet.infolist()
        build.require(len(members) == len(wanted) and {itm.filename for itm in members} == wanted,
                      "unexpected or duplicate guest result member")
        build.require(sum(member.file_size for member in members) <= MAX_RESULT, "guest result files exceed the bound")
        with tempfile.TemporaryDirectory(prefix="w11-guest-result-", dir=destination.parent) as scratch:
            shelf = Path(scratch) / "verified"
            shelf.mkdir()
            for member in members:
                name = member.filename
                build.require(member.compress_type == ZIP_STORED and member.file_size <= leaf_limit(name)
                              and member.compress_size == member.file_size
                              and stat.S_ISREG(member.external_attr >> 16), "unexpected guest result member shape")
                path = shelf / name
                path.parent.mkdir(parents=True, exist_ok=True)
                with path.open("xb") as file:
                    file.write(packet.read(member))
            raw_digest = verify_observations(shelf, subject)
            shelf.rename(destination)
    return {"sha256": sha256(body).hexdigest(), "raw_sha256": raw_digest, "files": len(wanted), "bytes": len(body)}


def protected_request(role, incoming):
    roles, _ = workflow.approved_plan()
    return request_bytes(roles[role], incoming, build.workflow_binding(), "protected")


def protected_copy_input(role, request, expected_digest, incoming, destination):
    roles, _ = workflow.approved_plan()
    return copy_input(request, expected_digest, roles[role], incoming,
                      build.workflow_binding(), "protected", destination)


def protected_receive_result(role, request, expected_digest, incoming, serial, destination):
    roles, _ = workflow.approved_plan()
    subject = check_request(request, expected_digest, roles[role], incoming,
                            build.workflow_binding(), "protected")
    return receive_result(request, serial, subject, destination)
