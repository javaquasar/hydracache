"""Local diagnostic observations only; no clock, configuration or file writes."""
import json
import sys
import time

first = [time.clock_gettime_ns(time.CLOCK_BOOTTIME), time.time_ns()]
last = first
refusals = []
for index in range(3000):
    current = [time.clock_gettime_ns(time.CLOCK_BOOTTIME), time.time_ns()]
    if current[0] < last[0] or current[1] < last[1]:
        refusals.append({"sample": index, "before": last, "after": current})
    last = current
    time.sleep(0.001)
print(json.dumps({"samples": 3000, "first": first, "last": last,
                  "backwards": refusals, "manual_clock_change": False}))
sys.exit(1 if refusals else 0)
