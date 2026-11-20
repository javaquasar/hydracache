#!/usr/bin/env python3
"""Verify and exercise the exact, previously attested 0.74 server ELFs.

The probe checks process ownership and RESP semantics. It produces no timings
for release admission and has no path to start a qualification campaign.
"""

import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import Path
import secrets
import shutil
import signal
import socket
import ssl
import stat
import subprocess
import time


RUN = "43785098748"
SIGNER = "65a559f550c13c61f2acbf5bcb72e70b21b600b9"
LOCK = "a03837132e90e3035e56149602227ce09367efb232ab3bf96d462f5282218271"
# Workspace package version of the two source trees pinned for this run.
ROLE_VERSION = b"0.73.0"
ROLES = {
    "I74": {
        "source": "2e2d488082a0e2d7778f22143c99e3beede628e7",
        "tree": "4d9da87f5f92b9c36ceeb21e7cdb0e48f69f0808",
        "elf": "5a644f9f041cfa6f19e8128f1184af99c0bfef8c2a166394ae113a449b0b7da9",
        "bytes": 22410416,
        "receipt": "0c5a0f68bb872cb15b4a4d5f926d60969f3742ee03f43847f35418a6dbf89968",
        "bundle": "6fd866dfdef80f7553394df8a4ba775844f4e876464a8dbdb908a7ef41529459",
    },
    "C74": {
        "source": "891bc4e89b62c2eba7449fcc3a7dac638175c438",
        "tree": "900697fafa7c6eb2c2304b6e6e9611507dec1d7e",
        "elf": "e45a5f7304f90a055f9b947118456ddf183ff4f8e709a966db43311748b27fd4",
        "bytes": 22411336,
        "receipt": "f72809f862d57ff7658747323025ad3ee455ee8d320531ef099d7df2b7894902",
        "bundle": "ecd90a6ba333f51684dfe4cb7895ec7fc21fb1c501bebc495afc91db757a1e9b",
    },
}
AX42_HOME = Path("/home/hydracache-admin")
AX42_BOOT = "e81671d6-bf23-4ebf-b8fe-5dad1f45866f"
CAMPAIGNS = Path("/var/lib/hydracache-performance/campaigns")
MARKERS = ("active-campaign", "active-diagnostic.json", ".active-diagnostic.pending",
           "diagnostic-requests.json", ".diagnostic-requests.pending")
DISK_FLOOR = 64 * 1024**3
CPU = 1
PROCESS_LIMIT = 60
STAGE_INDEX_SHA = "438c252ec97bdb46cec2c2a3181c504d800b387ecb1cbe4b67ee08c5e9f183e7"


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def read_pinned(path, expected, private=False, capture=False):
    info = path.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1,
            f"{path.name} must be a single regular file")
    if private:
        require((info.st_uid, info.st_gid, stat.S_IMODE(info.st_mode)) == (1000, 1000, 0o400),
                f"unexpected AX42 ownership or permissions for {path.name}")
    with path.open("rb") as stream:
        require(os.path.samestat(info, os.fstat(stream.fileno())), f"{path.name} changed before read")
        if capture:
            require(info.st_size <= 16 * 1024, f"oversized receipt {path.name}")
            body = stream.read(16 * 1024 + 1)
            require(len(body) == info.st_size, f"{path.name} changed size during read")
            digest = hashlib.sha256(body)
        else:
            body = None
            digest = hashlib.file_digest(stream, "sha256")
        require(digest.hexdigest() == expected, f"SHA-256 mismatch for {path.name}")
        after = path.lstat()
        require(os.path.samestat(info, after) and info.st_size == after.st_size and
                info.st_mtime_ns == after.st_mtime_ns and info.st_ctime_ns == after.st_ctime_ns and
                info.st_mode == after.st_mode and info.st_nlink == after.st_nlink,
                f"{path.name} changed during read")
    return info, body


def verify_stage(stage, private=False):
    info = stage.lstat()
    require(stat.S_ISDIR(info.st_mode), "stage must be a real directory")
    if private:
        require(stage == AX42_HOME / f"w11-signed-role-{RUN}-r1", "unexpected AX42 stage")
        require((info.st_uid, info.st_gid, stat.S_IMODE(info.st_mode)) == (1000, 1000, 0o700),
                "unexpected AX42 stage ownership or permissions")
    names = {role + suffix for role in ROLES for suffix in ("-server", "-receipt.json", "-sigstore.json")}
    require({entry.name for entry in stage.iterdir()} == names | {"SHA256SUMS"},
            "missing or extra staged files")
    read_pinned(stage / "SHA256SUMS", STAGE_INDEX_SHA, private)
    receipts = {}
    for role, pin in ROLES.items():
        binary = stage / f"{role}-server"
        info, _ = read_pinned(binary, pin["elf"], private)
        require(info.st_size == pin["bytes"], f"{role} ELF has an unexpected length")
        document = stage / f"{role}-receipt.json"
        _, body = read_pinned(document, pin["receipt"], private, capture=True)
        read_pinned(stage / f"{role}-sigstore.json", pin["bundle"], private)
        receipt = json.loads(body)
        require(receipt == {
            "schema": "hydracache-product-build-074-v1", "repository_id": "1217101761",
            "role": role, "source_commit": pin["source"], "source_tree": pin["tree"],
            "cargo_lock_sha256": LOCK, "package": "hydracache-server", "profile": "release",
            "rust_toolchain": "1.94.0", "target": "x86_64-unknown-linux-gnu",
            "workflow_commit": SIGNER, "run_id": RUN, "run_attempt": "1",
            "binary": {"name": binary.name, "sha256": pin["elf"], "bytes": pin["bytes"]},
        }, f"unexpected signed predicate for {role}")
        receipts[role] = receipt
    return receipts


def disk_guard(path):
    free = shutil.disk_usage(path).free
    require(free >= DISK_FLOOR, f"AX42 free bytes {free} below {DISK_FLOOR}")
    return free


def host_guard():
    require(os.getuid() == os.getgid() == 1000 and Path.home().resolve(strict=True) == AX42_HOME,
            "unexpected AX42 account")
    require(socket.gethostname() == "Ubuntu-noble-latest-amd64-base.zst" and
            Path("/proc/sys/kernel/random/boot_id").read_text().strip() == AX42_BOOT,
            "unexpected AX42 host or boot")
    require(Path("/sys/devices/system/cpu/isolated").read_text().strip() == "1-4" and
            Path("/sys/devices/system/cpu/cpu1/cpufreq/scaling_governor").read_text().strip() == "performance",
            "AX42 CPU isolation or governor changed")
    properties = ("MainPID", "NRestarts", "ActiveState", "SubState")
    command = ["/usr/bin/systemctl", "show", "hydracache-performance-supervisor-074.service"]
    for name in properties:
        command.extend(("-p", name))
    state = dict(line.split("=", 1) for line in subprocess.check_output(
        command, text=True, timeout=5).splitlines())
    require(state == dict(MainPID="1099074", NRestarts="0", ActiveState="active", SubState="running"),
            "AX42 supervisor identity changed")
    for name in MARKERS:
        for check in ("-e", "-L"):
            subprocess.run(["/usr/bin/sudo", "-n", "/usr/bin/test", "!", check, str(CAMPAIGNS / name)],
                           check=True, timeout=5)
    mount = subprocess.check_output(
        ["/usr/bin/findmnt", "-n", "-o", "FSTYPE", "--target", AX42_HOME], text=True, timeout=5)
    require(mount.strip() == "ext4", "AX42 workspace filesystem changed")
    return disk_guard(AX42_HOME)


def write_json(path, value):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2)
        stream.write("\n")


def fixture(path):
    path.mkdir(mode=0o700)
    (path / "server.ext").write_text("subjectAltName=DNS:localhost\nextendedKeyUsage=serverAuth\n")
    (path / "client.ext").write_text("extendedKeyUsage=clientAuth\n")
    command = ["/usr/bin/openssl"]
    key = ["-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1", "-noenc"]
    with (path / "openssl.log").open("xb") as log:
        def openssl(*args):
            subprocess.run(command + list(args), cwd=path, stdout=log, stderr=log, check=True, timeout=5,
                           env={"PATH": "/usr/bin:/bin", "LANG": "C"})

        openssl("req", "-x509", *key, "-days", "1", "-subj", "/CN=AX42 probe CA",
                "-keyout", "ca.key", "-out", "ca.crt", "-addext", "basicConstraints=critical,CA:TRUE",
                "-addext", "keyUsage=critical,keyCertSign,cRLSign")
        for name in ("server", "client"):
            common_name = "localhost" if name == "server" else "AX42 probe client"
            openssl("req", "-new", *key, "-subj", f"/CN={common_name}", "-keyout", f"{name}.key",
                    "-out", f"{name}.csr")
            openssl("x509", "-req", "-in", f"{name}.csr", "-CA", "ca.crt", "-CAkey", "ca.key",
                    "-CAcreateserial", "-days", "1", "-extfile", f"{name}.ext", "-out", f"{name}.crt")
    token = secrets.token_hex(24).encode("ascii")
    (path / "auth.token").write_bytes(token + b"\n")
    return token


def seconds_left(deadline):
    left = deadline - time.monotonic()
    if left <= 0:
        raise TimeoutError("signed role exceeded its process deadline")
    return left


def exchange(material, address, request, deadline, certificate=True):
    context = ssl.create_default_context(cafile=str(material / "ca.crt"))
    if certificate:
        context.load_cert_chain(material / "client.crt", material / "client.key")
    with socket.create_connection(address, timeout=min(5, seconds_left(deadline))) as conn:
        result = bytearray()
        try:
            with context.wrap_socket(conn, server_hostname="localhost", suppress_ragged_eofs=False) as tls:
                tls.settimeout(min(5, seconds_left(deadline)))
                tls.sendall(request)
                while True:
                    disk_guard(material)
                    tls.settimeout(min(5, seconds_left(deadline)))
                    data = tls.recv(4096)
                    if not data:
                        return bytes(result)
                    result.extend(data)
                    require(len(result) <= 4096, "oversized RESP probe reply")
        except ssl.SSLError as error:
            if not certificate and not result and error.reason in {
                "TLSV13_ALERT_CERTIFICATE_REQUIRED", "SSLV3_ALERT_HANDSHAKE_FAILURE",
            }:
                return b""
            if error.reason == "UNEXPECTED_EOF_WHILE_READING":
                # The peer may close TCP after QUIT. The caller still compares
                # the full, authenticated response to the exact wire oracle.
                return bytes(result)
            raise


def wire(*parts):
    return (b"*%d\r\n" % len(parts)) + b"".join(b"$%d\r\n%s\r\n" % (len(part), part) for part in parts)


def split_resp3_hello(reply, *, expected_version):
    # RESP3 maps are unordered. Keep exact key and value encodings and apply
    # positional comparison to the command replies following HELLO.
    header, *fields, replies = bytes(reply).split(b"\r\n", 15)
    expected = {
        b"+server": b"+hydracache", b"+version": b"+" + expected_version, b"+proto": b":3",
        b"+id": b":0", b"+mode": b"+standalone", b"+role": b"+master", b"+modules": b"*0",
    }
    require(header == b"%7" and len(fields) == 14 and
            dict(zip(fields[::2], fields[1::2])) == expected,
            "RESP3 HELLO map differed")
    return replies


def verify_resp3(reply, *, expected_version=ROLE_VERSION):
    replies = split_resp3_hello(reply, expected_version=expected_version)
    require(replies == b"+OK\r\n$3\r\n\0\xffb\r\n_\r\n+OK\r\n",
            "RESP3 value, absence, or reply order differed")


NOAUTH = b"-NOAUTH Authentication required.\r\n"
WRONGPASS = b"-WRONGPASS invalid username-password pair or user is disabled.\r\n"


def protocol(material, address, token, deadline, output, *, expected_version=ROLE_VERSION):
    quit_command = wire(b"QUIT")
    unexpected = wire(b"SET", b"denied", b"should not be written")
    authenticate = wire(b"AUTH", b"default", token)
    no_certificate = exchange(material, address, authenticate + unexpected + quit_command, deadline, certificate=False)
    (output / "no-certificate.resp").write_bytes(no_certificate)
    require(no_certificate == b"",
            "missing client certificate reached the RESP application")
    denied = exchange(material, address, unexpected + wire(b"AUTH", b"default", b"wrong-token") +
                      unexpected + quit_command, deadline)
    (output / "denied.resp").write_bytes(denied)
    require(denied == NOAUTH + WRONGPASS + NOAUTH + b"+OK\r\n", "incorrect AUTH refusal sequence")
    request = (wire(b"HELLO", b"3", b"AUTH", b"default", token) + wire(b"SET", b"probe", b"\0\xffb") +
               wire(b"GET", b"probe") + wire(b"GET", b"denied") + quit_command)
    reply = exchange(material, address, request, deadline)
    (output / "authenticated.resp").write_bytes(reply)
    verify_resp3(reply, expected_version=expected_version)
    fresh = exchange(material, address, wire(b"GET", b"probe") + quit_command, deadline)
    (output / "fresh-connection.resp").write_bytes(fresh)
    require(fresh == NOAUTH + b"+OK\r\n",
            "new TLS connections did not require AUTH")


def process_identity(child, image):
    require(child.poll() is None, f"daemon exited before identity check with {child.returncode}")
    proc = Path("/proc") / str(child.pid)
    require((proc / "exe").samefile(image) and os.sched_getaffinity(child.pid) == {CPU},
            "daemon ELF identity or CPU affinity differs")
    start_ticks = int((proc / "stat").read_text().rsplit(")", 1)[1].split()[19])
    return {"pid": child.pid, "start_ticks": start_ticks, "cpu": CPU}


def resp_environment(role_root, material, address):
    return {
        "PATH": "/usr/bin:/bin", "LANG": "C", "HOME": str(role_root), "TMPDIR": str(role_root),
        "TOKIO_WORKER_THREADS": "1", "HYDRACACHE_ROLE": "local",
        "HYDRACACHE_STORAGE_DIR": str(role_root / "storage"),
        "HYDRACACHE_MANAGEMENT_API_ENABLED": "false", "HYDRACACHE_ADMIN_API_ENABLED": "false",
        "HYDRACACHE_CLIENT_API_ENABLED": "false", "HYDRACACHE_HC2_ENABLED": "false",
        "HYDRACACHE_REDIS_API_ENABLED": "true", "HYDRACACHE_REDIS_ADDR": f"{address[0]}:{address[1]}",
        "HYDRACACHE_REDIS_AUTH_REQUIRED": "true", "HYDRACACHE_REDIS_AUTH_USERNAME": "default",
        "HYDRACACHE_REDIS_AUTH_TOKEN_FILE": str(material / "auth.token"),
        "HYDRACACHE_REDIS_REDISS_ENABLED": "true", "HYDRACACHE_TLS_ENABLED": "true",
        "HYDRACACHE_TLS_CERT_PATH": str(material / "server.crt"),
        "HYDRACACHE_TLS_KEY_PATH": str(material / "server.key"),
        "HYDRACACHE_TLS_CA_PATH": str(material / "ca.crt"),
        "HYDRACACHE_REDIS_MTLS_CLIENT_CA_PATH": str(material / "ca.crt"),
    }


@contextmanager
def signed_daemon(role, image, root, material, *, admin=False):
    role_root = root / role
    role_root.mkdir(mode=0o700)
    with socket.socket() as resp_port, socket.socket() as admin_port:
        resp_port.bind(("127.0.0.1", 0))
        admin_port.bind(("127.0.0.1", 0))
        address = resp_port.getsockname()
        admin_address = admin_port.getsockname()
    env = resp_environment(role_root, material, address)
    if admin:
        env["HYDRACACHE_ADMIN_API_ENABLED"] = "true"
        env["HYDRACACHE_ADMIN_ADDR"] = f"{admin_address[0]}:{admin_address[1]}"
    before = host_guard()
    read_pinned(image, ROLES[role]["elf"])
    deadline = time.monotonic() + PROCESS_LIMIT
    stdout = role_root / "stdout"
    with stdout.open("xb") as out, stdout.open("rb") as readiness, (role_root / "stderr").open("xb") as err:
        child = subprocess.Popen(["/usr/bin/taskset", "-c", str(CPU), str(image)], env=env, cwd=role_root,
                                 stdout=out, stderr=err, start_new_session=True)
        try:
            while True:
                seconds_left(deadline)
                disk_guard(root)
                require(child.poll() is None, f"daemon startup refused with {child.returncode}")
                readiness.seek(0)
                ready = readiness.readline(512)
                if ready.endswith(b"\n"):
                    require(ready == b'{"status":"ok"}\n', "unexpected daemon readiness response")
                    break
                time.sleep(0.05)
            identity = process_identity(child, image)
            outcome = {**identity, "sha256": ROLES[role]["elf"], "min_required_free_bytes": DISK_FLOOR,
                       "free_bytes_before": before}
            yield child, address, admin_address, deadline, outcome
            require(process_identity(child, image) == identity, "daemon process generation changed")
            child.send_signal(signal.SIGINT)
            require(child.wait(timeout=seconds_left(deadline)) == 0, "signed role failed to drain")
        finally:
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGKILL)
                child.wait(timeout=5)
    after = host_guard()
    read_pinned(image, ROLES[role]["elf"])
    outcome.update(exit=child.returncode, free_bytes_after=after)


def probe_role(role, image, root, material, token):
    with signed_daemon(role, image, root, material) as (_, address, _, deadline, outcome):
        protocol(material, address, token, deadline, root / role)
    return {**outcome, "mTLS": True, "auth_required_on_each_socket": True, "resp3_binary_round_trip": True}


def run_probe(stage, output):
    import resource

    require(output.is_absolute() and output.parent == AX42_HOME and output.name.startswith("w11-signed-probe-"),
            "AX42 probe requires a fresh private workspace directly under the AX42 home")
    os.umask(0o077)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    start_free = host_guard()
    require(start_free >= DISK_FLOOR + sum(pin["bytes"] for pin in ROLES.values()),
            "not enough space to stage the executable copies while retaining the reserve")
    receipts = verify_stage(stage, private=True)
    output.mkdir(mode=0o700)
    verdict = {"schema": "signed-role-probe-074-v1", "run": RUN, "attempt": "1", "boot": AX42_BOOT,
               "promotable": False, "qualified_pairs": 0, "roles": {}}
    try:
        material = output / "fixture"
        token = fixture(material)
        for role, build in receipts.items():
            image = output / build["binary"]["name"]
            with (stage / image.name).open("rb") as reader, image.open("xb") as writer:
                shutil.copyfileobj(reader, writer)
            read_pinned(image, ROLES[role]["elf"])
            image.chmod(0o500)
            verdict["roles"][role] = probe_role(role, image, output, material, token)
        verify_stage(stage, private=True)
        verdict["final_free_bytes"] = host_guard()
    except Exception as error:
        verdict["error"] = str(error)
        raise
    finally:
        write_json(output / "probe.json", verdict)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("stage", type=Path)
    parser.add_argument("--probe-ax42", type=Path, metavar="FRESH_OUTPUT")
    args = parser.parse_args()
    if args.probe_ax42 is None:
        print(json.dumps(verify_stage(args.stage), indent=2))
    else:
        run_probe(args.stage, args.probe_ax42)
        print((args.probe_ax42 / "probe.json").read_text(), end="")


if __name__ == "__main__":
    main()
