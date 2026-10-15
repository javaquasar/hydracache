"""Read only the fixed existing supervisor's public kernel/systemd context."""
import json
import os
from pathlib import Path
import subprocess

UNIT = "hydracache-performance-supervisor-074.service"
CGROUP = "/system.slice/" + UNIT
PROPERTIES = ("MainPID", "NRestarts", "ActiveState", "SubState", "ControlGroup",
              "User", "Group", "PrivateTmp", "PrivateUsers", "ProtectSystem",
              "ProtectHome", "ProtectControlGroups", "NoNewPrivileges", "KillMode")


def unit():
    result = subprocess.run(["/usr/bin/systemctl", "show", UNIT,
                             "--property=" + ",".join(PROPERTIES)],
                            env={"PATH": "/usr/sbin:/usr/bin:/sbin:/bin", "LANG": "C", "LC_ALL": "C"},
                            capture_output=True, timeout=5, check=True)
    if result.stderr or len(result.stdout) > 65536:
        raise ValueError("unit observation refused")
    values = dict(line.split("=", 1) for line in result.stdout.decode("ascii").splitlines())
    if set(values) != set(PROPERTIES) or values["ControlGroup"] != CGROUP:
        raise ValueError("unexpected original unit projection")
    return values


def bounded(name):
    with open(name, "rb") as stream:
        value = stream.read(65537)
    if not value or len(value) > 65536:
        raise ValueError("kernel observation budget refused")
    return value.decode("ascii")


def observe(pid):
    path = f"/proc/{pid}"
    raw = bounded(path + "/stat")
    if raw.split(" ", 1)[0] != str(pid):
        raise ValueError("process changed")
    fields = raw[raw.rindex(")") + 2:].split()
    start_ticks = int(fields[19])
    namespaces = {}
    for name in ("user", "mnt", "pid", "time"):
        location = path + "/ns/" + name
        info = os.stat(location)  # deliberate fixed proc namespace magic links
        namespaces[name] = {"device": info.st_dev, "inode": info.st_ino}
    return {"pid": pid, "start_ticks": start_ticks, "namespaces": namespaces,
            "cgroup": bounded(path + "/cgroup").strip()}


def collect():
    first = unit()
    pid = int(first["MainPID"])
    if os.geteuid() != 0 or pid <= 1 or first["ActiveState"] != "active":
        raise ValueError("fixed active root supervisor required")
    before, after = observe(pid), observe(pid)
    last = unit()
    if first != last or before != after or before["cgroup"] != "0::" + CGROUP:
        raise ValueError("unit or process changed during observation")
    root = Path("/sys/fs/cgroup") / CGROUP[1:]
    return {"schema_version": "diagnostic-enrollment-supervisor-inventory-074-v1",
            "unit_before": first, "unit_after": last, "process_before": before,
            "process_after": after, "cgroup_type": bounded(root / "cgroup.type").strip(),
            "original_cgroup_kill_file_present": (root / "cgroup.kill").is_file(),
            "read_only": True, "service_mutation": False,
            "workload_started": False, "host_admission": False,
            "numeric_pid_observation_not_original_start_or_cgroup_authority": True}


if __name__ == "__main__":
    print(json.dumps(collect(), sort_keys=True, indent=2))
