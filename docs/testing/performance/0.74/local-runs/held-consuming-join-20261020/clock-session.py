"""Keep one local WSL process alive; observe clocks without setting them."""
import json
import select
import sys
import time


def clocks():
    return [time.clock_gettime_ns(time.CLOCK_BOOTTIME), time.time_ns()]


def emit(event, **fields):
    print(json.dumps({"event": event, **fields}), flush=True)


phase = "warmup"
started = time.monotonic()
last = clocks()
backwards = []
emit("start", clocks=last, warmup_seconds=60, maximum_seconds=600)
ready = False
for sample in range(6000):
    current = clocks()
    if current[0] < last[0] or current[1] < last[1]:
        event = {"phase": phase, "sample": sample, "before": last, "after": current}
        backwards.append(event)
        emit("backwards", **event)
    last = current
    if not ready and time.monotonic() - started >= 60:
        ready = True
        emit("ready", clocks=current, warmup_backwards=len(backwards))
    if select.select([sys.stdin], [], [], 0)[0]:
        command = sys.stdin.readline()
        if not command or command.strip() == "finish":
            emit("finish", phase=phase, clocks=current, backwards=backwards)
            sys.exit(0)
        if command.strip() == "begin" and ready and phase == "warmup":
            phase = "tests"
            emit("begin", clocks=current)
        else:
            emit("invalid-command", phase=phase)
            sys.exit(2)
    if sample % 200 == 0:
        emit("progress", phase=phase, clocks=current)
    time.sleep(0.1)
emit("timeout", phase=phase, clocks=last, backwards=backwards)
sys.exit(3)
