"""One-shot public host inventory over SSH stdin; no enrollment or mutation.

No arguments, arbitrary paths, subprocesses, private keys, tokens, process
arguments or environment reads. Ordinary SSH/sudo audit logging and filesystem
access times are outside the no-configuration-mutation claim.
"""
import hashlib
import json
import os
from pathlib import Path
import platform
import stat
import time

LIMIT = 1048576
ACCOUNTS = ("hydracache-perf", "github-runner")
PUBLIC_FILES = ("/etc/passwd", "/etc/group", "/etc/nsswitch.conf",
                "/etc/machine-id", "/etc/hydracache-perf/worker-trust-074.json",
                "/etc/hydracache-perf/controller-trust-074.json",
                "/etc/hydracache-perf/oidc-trust-074.json")
PUBLIC_RUNTIME = ("/usr/lib/x86_64-linux-gnu/libc.so.6",
                  "/usr/lib/x86_64-linux-gnu/libgcc_s.so.1",
                  "/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2",
                  "/etc/ssl/certs/ca-certificates.crt")
DIRECTORIES = ("/etc/hydracache-perf", "/var/lib/hydracache-performance",
               "/var/lib/hydracache-performance/worker-authority-074",
               "/var/lib/hydracache-performance/diagnostic-intents-074",
               "/opt/hydracache-perf", "/opt/hydracache-perf/bin",
               "/sys/fs/cgroup")


def metadata(info):
    return {"device": info.st_dev, "inode": info.st_ino, "uid": info.st_uid,
            "gid": info.st_gid, "mode": oct(stat.S_IMODE(info.st_mode)),
            "type": stat.S_IFMT(info.st_mode), "links": info.st_nlink,
            "bytes": info.st_size, "mtime_ns": info.st_mtime_ns,
            "ctime_ns": info.st_ctime_ns}


def public_file(name, budget=LIMIT):
    """Descriptor-relative, no-follow public regular-file observation only."""
    handles = []
    try:
        current = os.open("/", os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
        handles.append(current)
        ancestry = [{"path": "/", **metadata(os.fstat(current))}]
        parts = Path(name).parts[1:]
        for index, part in enumerate(parts):
            directory = index != len(parts) - 1
            flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK
            if directory:
                flags |= os.O_DIRECTORY
            current = os.open(part, flags, dir_fd=current)
            handles.append(current)
            if directory:
                ancestry.append({"path": "/" + "/".join(parts[:index + 1]),
                                 **metadata(os.fstat(current))})
        before = os.fstat(current)
        if not stat.S_ISREG(before.st_mode) or before.st_size > budget:
            raise ValueError("public file shape refused")
        raw = bytearray()
        while block := os.read(current, min(65536, budget + 1 - len(raw))):
            raw.extend(block)
            if len(raw) > budget:
                raise ValueError("public file budget refused")
        if metadata(os.fstat(current)) != metadata(before) or len(raw) != before.st_size:
            raise ValueError("public file changed")
        return ({"status": "available", "metadata": metadata(before),
                 "ancestry": ancestry, "bytes": len(raw),
                 "sha256": hashlib.sha256(raw).hexdigest()}, bytes(raw))
    except FileNotFoundError:
        return {"status": "absent-at-read"}, None
    except (OSError, ValueError) as error:
        return {"status": "unknown", "reason": type(error).__name__}, None
    finally:
        for handle in reversed(handles):
            os.close(handle)


def proc_text(name):
    # Only fixed procfs paths below call this helper; proc files report size zero.
    with open(name, "rb") as stream:
        raw = stream.read(LIMIT + 1)
    if not raw or len(raw) > LIMIT:
        raise ValueError("proc observation budget refused")
    return raw.decode("ascii").strip()


def optional_kernel_text(name):
    try:
        return {"status": "available", "value": proc_text(name)}
    except FileNotFoundError:
        return {"status": "absent-at-read"}
    except (OSError, ValueError, UnicodeError) as error:
        return {"status": "unknown", "reason": type(error).__name__}


def context():
    namespaces = {}
    for process in ("1", "thread-self"):
        namespaces[process] = {}
        for name in ("user", "mnt", "pid", "time"):
            path = f"/proc/{process}/ns/{name}"
            # These fixed kernel namespace magic links are followed deliberately.
            info = os.stat(path)
            namespaces[process][name] = {"device": info.st_dev, "inode": info.st_ino,
                                         "link": os.readlink(path)}
    return {"boot_id": proc_text("/proc/sys/kernel/random/boot_id"),
            "namespaces": namespaces,
            "pid_one_uid_map": proc_text("/proc/1/uid_map"),
            "reader_uid_map": proc_text("/proc/thread-self/uid_map"),
            "pid_one_cgroup": proc_text("/proc/1/cgroup"),
            "reader_cgroup": proc_text("/proc/thread-self/cgroup"),
            "reader_root": metadata(os.stat("/")),
            "pid_one_root": metadata(os.stat("/proc/1/root"))}


def accounts(passwd, group):
    if passwd is None or group is None:
        return {"status": "unknown"}
    users = [line.split(":") for line in passwd.decode("utf-8").splitlines()]
    groups = [line.split(":") for line in group.decode("utf-8").splitlines()]
    result = {}
    for name in ACCOUNTS:
        matches = [row for row in users if row[0] == name]
        if len(matches) != 1 or len(matches[0]) != 7:
            result[name] = {"status": "absent-or-ambiguous"}
            continue
        uid, gid = int(matches[0][2]), int(matches[0][3])
        members = sorted(int(row[2]) for row in groups
                         if len(row) == 4 and name in row[3].split(","))
        result[name] = {"status": "available", "uid": uid, "gid": gid,
                        "explicit_supplementary_gids": members,
                        "group_ids_including_primary": sorted(set([gid, *members]))}
    named_groups = {}
    for name in ("hydracache-perf", "hydracache-perf-client", "github-runner"):
        matches = [row for row in groups if row[0] == name]
        named_groups[name] = ({"status": "available", "gid": int(matches[0][2]),
                               "audited_explicit_members": sorted(set(matches[0][3].split(",")) & set(ACCOUNTS))}
                              if len(matches) == 1 and len(matches[0]) == 4
                              else {"status": "absent-or-ambiguous"})
    return {"status": "available", "source": "local-files-not-NSS-or-process-proof",
            "accounts": result, "named_groups": named_groups}


def directory(name):
    try:
        info = Path(name).lstat()
        result = {"status": "present", "metadata": metadata(info)}
        if stat.S_ISDIR(info.st_mode) and name.endswith("/worker-authority-074"):
            entries = []
            with os.scandir(name) as iterator:
                for entry in iterator:
                    if len(entries) >= 256:
                        raise ValueError("authority directory roster budget refused")
                    entries.append({"name": entry.name,
                                    "metadata": metadata(entry.stat(follow_symlinks=False))})
            result["entries"] = sorted(entries, key=lambda entry: entry["name"])
            result["epoch_read_or_selected"] = False
        return result
    except FileNotFoundError:
        return {"status": "absent-at-read"}
    except (OSError, ValueError) as error:
        return {"status": "unknown", "reason": type(error).__name__}


def collect():
    if platform.system() != "Linux" or os.geteuid() != 0:
        raise ValueError("Linux root reader required; no privilege fallback")
    before = context()
    files, contents = {}, {}
    for name in PUBLIC_FILES:
        files[name], contents[name] = public_file(name)
    runtime = {name: public_file(name, 8388608)[0] for name in PUBLIC_RUNTIME}
    dirs = {name: directory(name) for name in DIRECTORIES}
    after = context()
    # Root directory size/times can change for unrelated entries. This report
    # retains both observations rather than treating them as stable authority.
    return {"schema_version": "diagnostic-enrollment-host-inventory-074-v1",
            "captured_at_unix_seconds": int(time.time()),
            "kernel": platform.release(), "effective_uid": os.geteuid(),
            "context_before": before, "context_after": after,
            "public_files": files, "directories": dirs,
            "public_runtime_files": runtime,
            "machine_id": contents["/etc/machine-id"].decode("ascii").strip()
                          if contents["/etc/machine-id"] is not None else None,
            "account_inventory": accounts(contents["/etc/passwd"], contents["/etc/group"]),
            "cgroup_controllers": optional_kernel_text("/sys/fs/cgroup/cgroup.controllers"),
            "cgroup_type": optional_kernel_text("/sys/fs/cgroup/cgroup.type"),
            "cgroup_kill_file_present": Path("/sys/fs/cgroup/cgroup.kill").is_file(),
            "system_slice_cgroup_type": optional_kernel_text("/sys/fs/cgroup/system.slice/cgroup.type"),
            "system_slice_cgroup_kill_present": Path("/sys/fs/cgroup/system.slice/cgroup.kill").is_file(),
            "read_only": True, "host_admission": False, "policy_epoch_selected": False,
            "policy_signed": False, "service_mutation": False,
            "workload_started": False, "namespace_initial_host_attested": False}


if __name__ == "__main__":
    print(json.dumps(collect(), sort_keys=True, indent=2))
