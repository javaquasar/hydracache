"""Read clocks only; boot identity is read by the invoking shell, not authority."""
import json
import os
import subprocess
import time

boot = subprocess.check_output(["cat", "/proc/sys/kernel/random/boot_id"], text=True).strip()
assert len(boot) == 36 and all(c in "0123456789abcdef-" for c in boot)
for sample in range(180):
    print(json.dumps({
        "sample": sample, "boot": boot, "pid": os.getpid(),
        "realtime_ns": time.clock_gettime_ns(time.CLOCK_REALTIME),
        "boottime_ns": time.clock_gettime_ns(time.CLOCK_BOOTTIME),
        "monotonic_ns": time.clock_gettime_ns(time.CLOCK_MONOTONIC),
        "raw_ns": time.clock_gettime_ns(time.CLOCK_MONOTONIC_RAW),
    }, separators=(",", ":")), flush=True)
    if sample != 179:
        time.sleep(0.5)
