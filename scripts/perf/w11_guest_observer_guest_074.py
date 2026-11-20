"""Run the existing protected observer from the offline guest's read-only seed."""

from hashlib import sha256
import os
from pathlib import Path
import socket
import subprocess
import sys

from . import w11_build_observer_074 as observer
from . import w11_guest_artifact_transfer_074 as transfer


require = transfer.build.require


def guest_environment(seed, work):
    require(os.getuid() == 65534 and os.getgid() == 65534, "unexpected guest account")
    require(os.sched_getaffinity(0) == {0, 1, 2}, "guest CPUs 0, 1 and 2 are required")
    require({name for _, name in socket.if_nameindex()} == {"lo"}, "unexpected guest network interface")
    require(os.statvfs(seed).f_flag & os.ST_RDONLY, "guest input must be read-only")
    require(work.is_dir() and work.stat().st_mode & 0o077 == 0, "guest work directory is not private")
    observer.probe.disk_guard(work)
    observer.run_fixed(*observer.NAMESPACE, "/usr/bin/true")


def observe(role, seed, work, request_sha256, context_sha256):
    guest_environment(seed, work)
    context = transfer.receipt.read_json_bytes(seed / "context.json")
    require(sha256(context).hexdigest() == context_sha256, "guest workflow context SHA-256 differs")
    environment = transfer.build.decode_json(context)
    require(isinstance(environment, dict) and set(environment) == set(transfer.BUILD_CONTEXT_KEYS)
            and all(isinstance(value, str) for value in environment.values()), "unexpected guest workflow context")
    os.environ.update(environment)
    request = transfer.receipt.read_json_bytes(seed / transfer.REQUEST)
    incoming = work / "verified"
    observations = work / "observations"
    subject = transfer.protected_copy_input(role, request, request_sha256, seed / "incoming", incoming)
    print("W11_OBSERVER_BEGIN " + request_sha256, flush=True)
    subprocess.run(
        [sys.executable, "-B", "-m", "scripts.perf.w11_build_observer_074",
         role, str(incoming), str(observations)],
        check=True, cwd=seed, env={**environment, "PATH": "/usr/bin:/bin", "LANG": "C.UTF-8"},
    )
    print("W11_OBSERVER_END " + request_sha256, flush=True)
    body = transfer.result_bytes(observations, subject)
    sys.stdout.buffer.write(transfer.frame_response(request, body))
    sys.stdout.buffer.flush()
    observer.probe.disk_guard(work)


if __name__ == "__main__":
    role, seed, work, request_sha256, context_sha256 = sys.argv[1:]
    observe(role, Path(seed), Path(work), request_sha256, context_sha256)
