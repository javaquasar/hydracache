"""Observe one protected v2 build in the admitted, offline Debian guest."""

import argparse
from base64 import b64encode
from hashlib import file_digest, sha256
import os
from pathlib import Path
import re
import signal
import stat
import subprocess
import tempfile
import time

from . import w11_guest_artifact_transfer_074 as transfer
from . import performance_signed_role_probe_074 as probe


require = transfer.build.require
workflow = transfer.workflow
runtime_var = "HYDRACACHE_W11_OBSERVER_RUNTIME"
PROCESS_SECONDS = probe.PROCESS_LIMIT
OBSERVER_SECONDS = 2 * PROCESS_SECONDS + 30
BOOT_SECONDS = 95
OBSERVATION_ENVELOPE_SECONDS = OBSERVER_SECONDS + 5
RETURN_SECONDS = 20
JOB_SECONDS = 5 * 60
UPLOAD_SECONDS = 10
VM_SECONDS = BOOT_SECONDS + OBSERVATION_ENVELOPE_SECONDS + RETURN_SECONDS
MIN_STAGING = 1025 * 1024**2
PARTUUID = "3612a9bf-c7cf-40cb-9789-6117e94b7db6"
BASE_SHA512 = ("d25999de0770d4cde1be72c299ba3ad94eac7ea8fd4d18f8cc86c69df69337e2e8"
               "a91d64465a9bfd8f2a78c566fcbcb0842eacc91c09efdef72d719d9f13b535")
GUEST_ARGUMENTS = (
    f"root=PARTUUID={PARTUUID} ro console=ttyS0,115200 earlyprintk=ttyS0,115200 consoleblank=0"
    " systemd.mask=systemd-networkd-wait-online.service"
)
RUNTIME_FILES = ("debian-base.qcow2", "pinned-boot/vmlinuz", "pinned-boot/initrd")
RUNTIME_TOOLS = (
    "tools/usr/bin/qemu-system-x86_64", "tools/usr/bin/qemu-img", "tools/usr/bin/genisoimage",
    "tools/usr/share/seabios/bios-256k.bin",
)
SEED_FILES = (
    "performance_signed_role_probe_074.py", "product_build_receipt_074.py",
    "w11-product-build-approval-074.json", "w11_batch_boundary_074.py", "w11_batch_probe_074.py",
    "w11_build_observer_074.py", "w11_build_workflow_074.py", "w11_config_readback_074.py",
    "w11_guest_artifact_transfer_074.py", "w11_guest_observer_guest_074.py", "w11_product_build_receipt_074.py",
)


def digest(path, algorithm="sha256"):
    with path.open("rb") as stream:
        return file_digest(stream, algorithm).hexdigest()


def runtime_inventory(root):
    workflow.require_directory(root)
    workflow.require_directory(root / "tools")
    require(re.fullmatch(r"/[A-Za-z0-9_./-]+", str(root)) is not None, "unsupported runtime path")
    files = {}
    for path in sorted([*(root / "tools").rglob("*"), *(root / key for key in RUNTIME_FILES)]):
        name = path.relative_to(root).as_posix()
        info = path.lstat()
        if stat.S_ISLNK(info.st_mode):
            files[name] = {"link": os.readlink(path)}
        elif stat.S_ISREG(info.st_mode):
            require(path.resolve() == path, "runtime file has a linked parent")
            files[name] = {"mode": stat.S_IMODE(info.st_mode), "bytes": info.st_size, "sha256": digest(path)}
        else:
            require(stat.S_ISDIR(info.st_mode), "unexpected runtime entry")
    for name in (*RUNTIME_FILES, *RUNTIME_TOOLS):
        require("sha256" in files.get(name, {}), f"missing regular runtime asset {name}")
    require(digest(root / RUNTIME_FILES[0], "sha512") == BASE_SHA512, "unexpected Debian base bytes")
    return files


def check_runtime(root, expected):
    inventory = runtime_inventory(root)
    require(sha256(transfer.receipt.canonical(inventory)).hexdigest() == expected,
            "offline observer runtime differs from its approval")
    return inventory


def host_guard(directory):
    require(os.getuid() != 0 and {0, 1, 2} <= os.sched_getaffinity(0),
            "an unprivileged observer with CPUs 0, 1 and 2 is required")
    free = probe.disk_guard(directory)
    units = subprocess.check_output(
        ["/usr/bin/systemctl", "list-units", "--all", "--full", "--type=service", "--no-legend",
         "--no-pager", "hydracache-performance-074-*.service"], text=True, timeout=5,
    )
    require(all(line.split()[1:4] == ["loaded", "active", "exited"] for line in units.splitlines() if line.strip()),
            "an AX42 qualification unit is active or changed")
    return free


def job_deadline(clock):
    raw = clock.read_bytes()
    require(re.fullmatch(rb"[1-9][0-9]{0,19}\n", raw) is not None, "unexpected observer job clock")
    start = int(raw) / 10**9
    require(start <= time.monotonic() < start + JOB_SECONDS, "observer job clock expired")
    return start + JOB_SECONDS


def can_boot(deadline, now):
    require(now + VM_SECONDS + UPLOAD_SECONDS <= deadline,
            "insufficient job time for the original observation window and upload")


def boot_context(role, request_sha256, context_sha256):
    require(role in transfer.build.ROLES and all(transfer.receipt.is_hex(pin, 64)
            for pin in (request_sha256, context_sha256)), "invalid guest start context")
    return (
        "#!/bin/bash\nset -Eeuo pipefail\nexec >/dev/ttyS1 2>&1\n"
        "trap 'code=$?; printf \"\\nW11_GUEST_EXIT=%d\\n\" \"$code\"; sync; systemctl poweroff' EXIT\n"
        "version=$(systemctl --version)\nversion=${version%%$'\\n'*}\n"
        "[[ $version == 'systemd 257 '* || $version == 'systemd 257.'* ]]\n"
        "[[ $(systemctl show -p LoadState --value systemd-networkd-wait-online.service) == masked ]]\n"
        "printf 'W11_SYSTEMD=%s\\n' \"$version\"\n"
        "install -d -m 0555 /run/w11-media\n"
        "install -d -o 65534 -g 65534 -m 0700 /var/tmp/w11-observer\n"
        "mount -o ro,nosuid,nodev,noexec LABEL=cidata /run/w11-media\n"
        "cd /run/w11-media/transfer\n"
        "setpriv --reuid 65534 --regid 65534 --clear-groups -- "
        "/usr/bin/python3 -B -m scripts.perf.w11_guest_observer_guest_074 "
        f"{role} /run/w11-media/transfer /var/tmp/w11-observer {request_sha256} {context_sha256}\n"
    ).encode()


def cloud_seed(media, script, instance):
    (media / "meta-data").write_text(f"instance-id: {instance}\nlocal-hostname: w11-observer\n")
    (media / "network-config").write_text("version: 2\nethernets: {}\n")
    (media / "user-data").write_text(
        "#cloud-config\nusers: []\nssh_pwauth: false\ndisable_root: true\n"
        "growpart:\n  mode: auto\n  devices: ['/']\n  ignore_growroot_disabled: true\n"
        "resize_rootfs: true\nwrite_files:\n  - path: /root/w11-observer.sh\n"
        "    owner: root:root\n    permissions: '0700'\n"
        "    encoding: b64\n    content: " + b64encode(script).decode("ascii") +
        "\nruncmd:\n  - [systemd-run, --unit=w11-observer, --on-active=5s, "
        "--timer-property=AccuracySec=1s, /bin/bash, /root/w11-observer.sh]\n"
    )


def prepare_seed(role, incoming, media, request=None):
    if request is None:
        request = transfer.protected_request(role, incoming)
    request_digest = sha256(request).hexdigest()
    seed = media / "transfer"
    scripts = seed / "scripts/perf"
    scripts.mkdir(mode=0o700, parents=True)
    for item in SEED_FILES:
        (scripts / item).write_bytes(Path(__file__).with_name(item).read_bytes())
    transfer.protected_copy_input(role, request, request_digest, incoming, seed / "incoming")
    (seed / transfer.REQUEST).write_bytes(request)
    context = transfer.receipt.canonical({key: os.environ[key] for key in transfer.BUILD_CONTEXT_KEYS}) + b"\n"
    (seed / "context.json").write_bytes(context)
    cloud_seed(media, boot_context(role, request_digest, sha256(context).hexdigest()),
               "w11-" + request_digest[:16])
    return request, request_digest


def vm_arguments(root, workspace, media):
    bins = root / "tools/usr/bin"
    env = {"PATH": "/usr/bin:/bin", "LANG": "C.UTF-8", "HOME": str(workspace),
           "LD_LIBRARY_PATH": str(root / "tools/usr/lib/x86_64-linux-gnu"),
           "QEMU_MODULE_DIR": str(root / "tools/usr/lib/x86_64-linux-gnu/qemu")}
    overlay = workspace / "guest.qcow2"
    seed = workspace / "seed.iso"
    subprocess.run([bins / "qemu-img", "create", "-f", "qcow2", "-F", "qcow2", "-b", root / RUNTIME_FILES[0],
                    overlay, "96G"], check=True, env=env, stdout=subprocess.DEVNULL, timeout=12)
    subprocess.run([bins / "genisoimage", "-output", seed, "-volid", "cidata", "-joliet", "-r", media],
                   check=True, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=12)
    args = [bins / "qemu-system-x86_64", "-L", root / "tools/usr/share/qemu", "-bios", root / RUNTIME_TOOLS[-1],
            "-machine", "q35", "-accel", "tcg", "-cpu", "max", "-smp", "3", "-m", "2048",
            "-nodefaults", "-nic", "none", "-monitor", "none", "-display", "none", "-no-reboot",
            "-serial", f"file:{workspace}/console.log", "-serial", f"file:{workspace}/serial.log",
            "-drive", f"if=none,id=system,file={overlay},format=qcow2",
            "-device", "virtio-blk-pci,drive=system,bootindex=1",
            "-drive", f"if=none,id=seed,file={seed},format=raw,readonly=on",
            "-device", "virtio-blk-pci,drive=seed",
            "-kernel", root / RUNTIME_FILES[1], "-initrd", root / RUNTIME_FILES[2], "-append", GUEST_ARGUMENTS]
    return args, env


def serial_state(raw, request_sha256):
    context = request_sha256.encode("ascii")
    lines = raw.splitlines()
    marks = [line for line in lines if line.startswith((b"W11_OBSERVER_BEGIN ", b"W11_OBSERVER_END "))]
    expected = [b"W11_OBSERVER_BEGIN " + context, b"W11_OBSERVER_END " + context]
    require(marks == expected[:len(marks)], "guest observation marks differ from the request")
    exits = [line for line in lines if line.startswith(b"W11_GUEST_EXIT=")]
    require(len(exits) <= 1 and all(line == b"W11_GUEST_EXIT=0" for line in exits), "guest reported failure")
    return len(marks), bool(exits)


def serial_read(workspace):
    port = workspace / "serial.log"
    if not port.is_file():
        return b""
    with port.open("rb") as stream:
        data = stream.read(transfer.MAX_SERIAL + 1)
    require(len(data) <= transfer.MAX_SERIAL, "guest serial log exceeded its bound")
    return data


def guest_phase(workspace, request_sha256, deadline, started, phases):
    raw = serial_read(workspace)
    # A concurrently written final line is only considered after its delimiter.
    complete = raw[:raw.rfind(b"\n") + 1]
    mark, _ = serial_state(complete, request_sha256)
    now = time.monotonic()
    require(now <= deadline, "offline observer job deadline expired")
    require(mark >= len(phases), "guest observation marks disappeared")
    while len(phases) < mark:
        phases.append(now)
    beginning = phases[0] if phases else now
    require(beginning - started <= BOOT_SECONDS, "offline guest boot exceeded its window")
    if phases:
        ending = phases[1] if len(phases) == 2 else now
        require(ending - beginning <= OBSERVATION_ENVELOPE_SECONDS, "guest observation exceeded its window")
    if len(phases) == 2:
        require(now - phases[1] <= RETURN_SECONDS, "guest return or shutdown exceeded its window")
    return raw


def run_vm(root, workspace, media, request_sha256, deadline, guard=host_guard):
    args, env = vm_arguments(root, workspace, media)
    guard(workspace)
    started = time.monotonic()
    can_boot(deadline, started)
    phases = []
    with (workspace / "qemu.log").open("xb") as output:
        process = subprocess.Popen(args, env=env, stdout=output, stderr=output, start_new_session=True,
                                   preexec_fn=lambda: os.sched_setaffinity(0, {0, 1, 2}))
        try:
            while process.poll() is None:
                guard(workspace)
                guest_phase(workspace, request_sha256, deadline - UPLOAD_SECONDS, started, phases)
                time.sleep(0.5)
            raw = guest_phase(workspace, request_sha256, deadline - UPLOAD_SECONDS, started, phases)
            finished = time.monotonic()
            require(process.returncode == 0 and serial_state(raw, request_sha256) == (2, True),
                    "offline guest failed or omitted completion markers")
        finally:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                try:
                    process.wait(timeout=4)
                except subprocess.TimeoutExpired:
                    os.killpg(process.pid, signal.SIGKILL)
                    process.wait(timeout=2)
    guard(workspace)
    return raw, (phases[0] - started, phases[1] - phases[0], finished - phases[1])


def admission(directory):
    _, pin = workflow.approved_plan()
    transfer.build.workflow_binding()
    runtime = Path(os.environ[runtime_var])
    check_runtime(runtime, pin)
    require(host_guard(directory) >= probe.DISK_FLOOR + MIN_STAGING,
            "insufficient staging space above the 64 GiB reserve")
    return runtime, pin


def interrupted(signum, _frame):
    raise InterruptedError(f"offline observer interrupted by signal {signum}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("clock", type=Path)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--role", choices=transfer.build.ROLES)
    inputs = parser.parse_args()
    os.umask(0o077)
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGHUP, interrupted)
    deadline = job_deadline(inputs.clock)
    runtime, pin = admission(inputs.directory)
    if inputs.role is None:
        can_boot(deadline, time.monotonic())
        return
    incoming = inputs.directory / "build"
    destination = inputs.directory / "observations"
    scope = inputs.directory.resolve(strict=True)
    require(re.fullmatch(r"/[A-Za-z0-9_./-]+", str(scope)) is not None, "unsupported staging path")
    with tempfile.TemporaryDirectory(prefix="w11-observer-", dir=scope) as scratch:
        workspace = Path(scratch)
        media = workspace / "media"
        request, request_digest = prepare_seed(inputs.role, incoming, media)
        raw, times = run_vm(runtime, workspace, media, request_digest, deadline)
        check_runtime(runtime, pin)
        receipt = transfer.protected_receive_result(inputs.role, request, request_digest, incoming, raw, destination)
    print(f"offline observer phases: {times}; verified archive: {receipt['raw_sha256']}")


if __name__ == "__main__":
    main()
