"""Observe a frozen W11 build once per profile in a private Linux namespace."""

import argparse
from contextlib import contextmanager
import os
from pathlib import Path
import resource
import shutil
import signal
import socket
import subprocess
import sys
import tempfile
import time

from . import performance_signed_role_probe_074 as probe
from . import product_build_receipt_074 as build
from . import w11_batch_boundary_074 as batch
from . import w11_batch_probe_074 as joined
from . import w11_build_workflow_074 as workflow
from .w11_build_workflow_074 import CLIENT_CPU, DAEMON_CPU
from . import w11_config_readback_074 as readback
from . import w11_product_build_receipt_074 as receipt


BOOTSTRAP = ("/usr/bin/setpriv", "--no-new-privs", "--bounding-set=-all",
             "--inh-caps=-all", "--ambient-caps=-all", "--")
NAMESPACE = ("/usr/bin/unshare", "--user", "--map-root-user", "--mount", "--propagation", "private",
             "--net", "--pid", "--fork", "--kill-child=KILL", "--mount-proc", "--")
TRUST_FILES = ("ca.crt", "server.crt", "server.key", "auth.token")


def namespace_ids():
    return [Path(f"/proc/thread-self/ns/{kind}").stat().st_ino for kind in ("mnt", "pid", "net")]


def preflight(directory, staging_bytes=0):
    build.require(os.getuid() != 0, "start observation as the admitted unprivileged runner")
    build.require({CLIENT_CPU, DAEMON_CPU} <= os.sched_getaffinity(0), "CPU 1/2 are required")
    roles, _ = workflow.approved_plan()
    binding = build.workflow_binding()
    build.require(probe.disk_guard(directory) - probe.DISK_FLOOR >= staging_bytes,
                  "insufficient staging space above the 64 GiB reserve")
    if staging_bytes:
        run_fixed(*NAMESPACE, "/usr/bin/true")
    return roles, binding


def run_fixed(*argv):
    subprocess.run(argv, check=True, timeout=5, env={"PATH": "/usr/bin:/bin", "LANG": "C"})


@contextmanager
def private_root(root, original, subject, material):
    root.mkdir(mode=0o700)
    for name in ("usr/lib", "usr/bin", "lib64", "material", "img"):
        (root / name).mkdir(mode=0o700, parents=True, exist_ok=True)
    (root / "lib").symlink_to("usr/lib")
    image = root / "img" / original.name
    copies = [
        (original, image, 0o500),
        (Path("/usr/bin/setpriv"), root / "usr/bin/setpriv", 0o500),
        (Path("/lib64/ld-linux-x86-64.so.2"), root / "lib64/ld-linux-x86-64.so.2", 0o500),
        *((material / name, root / "material" / name, 0o400) for name in TRUST_FILES),
    ]
    for source, target, mode in copies:
        with source.open("rb") as incoming, target.open("xb") as installed:
            shutil.copyfileobj(incoming, installed)
        target.chmod(mode)
    probe.read_pinned(image, subject["binary"]["sha256"])
    libs = root / "usr/lib"
    run_fixed("/usr/bin/mount", "--bind", "/usr/lib", str(libs))
    try:
        run_fixed("/usr/bin/mount", "-o", "remount,bind,ro,nodev,nosuid", str(libs))
        yield image
    finally:
        run_fixed("/usr/bin/umount", str(libs))


def readiness(child, output, disk, deadline):
    while True:
        probe.seconds_left(deadline)
        probe.disk_guard(disk)
        build.require(child.poll() is None, "original ELF exited during startup")
        output.seek(0)
        data = output.readline(len(readback.READY) + 1)
        if data.endswith(b"\n") or len(data) > len(readback.READY):
            build.require(data == readback.READY, "original readiness bytes differ")
            return data
        time.sleep(0.02)


def observe_profile(subject, image, root, material, token, selector, output, boot, namespace):
    case = root / "case"
    case.mkdir(mode=0o700)
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as port:
        port.bind(("127.0.0.1", 0))
        address = port.getsockname()
    environment = probe.resp_environment(Path("/case"), Path("/material"), address)
    environment["HYDRACACHE_W11_CONFIG_READBACK"] = "1"
    if selector is not None:
        environment["HYDRACACHE_CLIENT_BATCH_PROFILE"] = selector

    def enter():
        os.sched_setaffinity(0, {DAEMON_CPU})
        os.chroot(root)
        os.chdir("/case")

    stdout = case / "stdout"
    with readback.private_channel(case) as channel, stdout.open("xb") as writer, stdout.open("rb") as reader:
        started = time.monotonic_ns()
        deadline = (started + probe.PROCESS_LIMIT * 10**9) / 10**9
        with subprocess.Popen(
            [*BOOTSTRAP, f"/img/{image.name}"], env=environment, preexec_fn=enter,
            stdin=subprocess.DEVNULL, stdout=writer, stderr=subprocess.DEVNULL, start_new_session=True,
        ) as child:
            try:
                ready = readiness(child, reader, case, deadline)
                build.require(os.sched_getaffinity(0) == {CLIENT_CPU}
                              and os.sched_getaffinity(child.pid) == {DAEMON_CPU}, "proof CPU masks differ")
                session = batch.RespSession(material, address, token, deadline, workflow.ROLE_VERSION)
                record = joined.collect_generation(subject, child, image, boot, namespace,
                                                   channel, ready, selector, session, output)
                build.require(os.sched_getaffinity(child.pid) == {DAEMON_CPU}, "daemon CPU mask changed")
                child.send_signal(signal.SIGINT)
                build.require(child.wait(timeout=probe.seconds_left(deadline)) == 0,
                              "original daemon did not drain successfully")
                probe.seconds_left(deadline)
                elapsed = time.monotonic_ns() - started
                build.require(0 < elapsed <= probe.PROCESS_LIMIT * 10**9, "proof process exceeded its deadline")
                result = {"daemon_pid": child.pid, "client_cpu": CLIENT_CPU, "daemon_cpu": DAEMON_CPU,
                          "elapsed_ns": elapsed, "exit": child.returncode}
                batch.write_bytes(output / "run.json", receipt.canonical(result) + b"\n")
            finally:
                if child.poll() is None:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait(timeout=3)
    probe.read_pinned(image, subject["binary"]["sha256"])
    return record


def collect(role, incoming, observations, outer):
    build.require(os.getpid() == 1 and os.getuid() == 0
                  and all(current != old for current, old in zip(namespace_ids(), outer, strict=True)),
                  "a private PID, mount and network namespace is required")
    build.require({CLIENT_CPU, DAEMON_CPU} <= os.sched_getaffinity(0), "CPU 1/2 are required")
    probe.disk_guard(observations.parent)
    os.umask(0o077)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    run_fixed("/usr/sbin/ip", "link", "set", "dev", "lo", "up")
    build.require({name for _, name in socket.if_nameindex()} == {"lo"}, "proof namespace has network access")
    os.sched_setaffinity(0, {CLIENT_CPU})
    roles, _ = workflow.approved_plan()
    binding = build.workflow_binding()
    subject = workflow.check_build(roles[role], incoming, binding)
    original = incoming / subject["binary"]["name"]
    build.require(probe.disk_guard(observations.parent) - probe.DISK_FLOOR >= 2 * subject["binary"]["bytes"],
                  "insufficient space for both isolated image copies")
    observations.mkdir(mode=0o700)
    archives = observations / "archives"
    archives.mkdir(mode=0o700)
    batch.write_bytes(archives / workflow.SUBJECT, receipt.canonical(subject) + b"\n")
    boot = readback.BOOT_ID.read_text().strip()
    namespace = joined.namespace_identity("thread-self")
    with tempfile.TemporaryDirectory(prefix="w11-", dir=observations.parent) as scratch:
        scratch = Path(scratch)
        material = scratch / "certs"
        token = probe.fixture(material)
        records = []
        for label, selector in ((batch.DEFAULT, None), (receipt.SELECTOR, receipt.SELECTOR)):
            jail = scratch / label
            with private_root(jail, original, subject, material) as image:
                records.append(observe_profile(subject, image, jail, material, token, selector,
                                               archives / label, boot, namespace))
        batch.publish_evidence(subject, *records, archives, observations)
    workflow.check_build(roles[role], incoming, binding)
    probe.disk_guard(observations)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("role", choices=build.ROLES)
    parser.add_argument("incoming", type=Path)
    parser.add_argument("observations", type=Path)
    parser.add_argument("--inside", type=int, nargs=3)
    args = parser.parse_args()
    if args.inside is not None:
        collect(args.role, args.incoming, args.observations, args.inside)
        return
    roles, binding = preflight(args.observations.parent)
    workflow.check_build(roles[args.role], args.incoming, binding)
    outer = [str(value) for value in namespace_ids()]
    environment = {key: os.environ[key] for key in (
        "GITHUB_REPOSITORY", "GITHUB_REPOSITORY_ID", "GITHUB_REF", "GITHUB_WORKFLOW_REF",
        "GITHUB_SHA", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT",
    )}
    environment.update(PATH="/usr/bin:/bin", LANG="C.UTF-8")
    subprocess.run(
        [*NAMESPACE, sys.executable,
         "-B", "-m", "scripts.perf.w11_build_observer_074", args.role, str(args.incoming),
         str(args.observations), "--inside", *outer],
        check=True, env=environment, timeout=2 * probe.PROCESS_LIMIT + 30,
    )


if __name__ == "__main__":
    main()
