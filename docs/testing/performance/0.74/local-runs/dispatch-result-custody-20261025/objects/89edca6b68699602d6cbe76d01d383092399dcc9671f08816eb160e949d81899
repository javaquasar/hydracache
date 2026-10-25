"""One-shot dispatch bookkeeping and process-local observation custody.

Not production enrollment, durable custody, an all-writer fence or authority to
execute Cargo. A caller owns the single spawn callback and the original owner.
"""

import copy
import ctypes
import json
import os
from pathlib import Path
import re

from owned_process_observer import FileTime, Refusal
from process_lineage import SnapshotApi


class ExitApi(SnapshotApi):
    def exit_witness(self, handle):
        """Use only a retained handle. No census, reopening, I/O or memory query."""
        status = self.dll.WaitForSingleObject(handle, 0)
        if status == 258:
            return None
        if status != 0:
            raise Refusal(f"exit witness wait: {status}")
        pid = self.dll.GetProcessId(handle)
        self.require(pid, "exit witness GetProcessId")
        created, exited, unused_kernel, unused_user = (FileTime() for _ in range(4))
        self.require(self.dll.GetProcessTimes(handle, ctypes.byref(created), ctypes.byref(exited),
                                             ctypes.byref(unused_kernel), ctypes.byref(unused_user)),
                     "exit witness GetProcessTimes")
        return dict(pid=pid, creation_100ns=created.ticks(), exit_100ns=exited.ticks())


class ObservationCustody:
    """Freeze telemetry on refusal; separately prove termination of held processes."""

    def __init__(self, owner, root, monitor, api):
        if monitor.owner is not owner or monitor.root is not root or root.handle is None:
            raise Refusal("custody is not the original owner/root")
        self.owner, self.root, self.monitor, self.api = owner, root, monitor, api
        self.observation = None
        self.first_refusal = None
        self.custody_refusal = None
        self.exit_proof = None
        self.release_attempted = False
        self.released = False

    def freeze(self, reason=None):
        if self.observation is not None:
            return
        if reason is not None:
            self.monitor.refuse(reason)
        self.first_refusal = self.monitor.first_refusal
        self.observation = copy.deepcopy(self.monitor.result())

    def tick(self, elapsed):
        if self.release_attempted or self.custody_refusal is not None:
            raise Refusal("custody cannot be retried after refusal/release attempt")
        if self.observation is None:
            try:
                self.monitor.tick(elapsed)
            except Exception as error:
                self.freeze(error)
            if self.monitor.first_refusal is not None or self.monitor.owner_exit_observed or self.monitor.outer_expired:
                self.freeze()
        if self.observation is not None:
            self.drain()

    def drain(self):
        if self.custody_refusal is not None:
            raise Refusal("exit-witness failure cannot be retried to pass")
        if self.exit_proof is not None:
            return True
        try:
            code = self.owner.poll()
            if code is None:
                return False
            handles = [self.root.handle]
            identities = [dict(pid=self.root.pid, creation_100ns=self.root.samples[0]["creation_100ns"])]
            if self.monitor.chain is not None:
                handles += self.monitor.chain.handles
                identities += self.monitor.chain.identities[1:]
            if len(handles) != len(identities) or any(handle is None for handle in handles):
                raise Refusal("held custody coverage changed")
            witnesses = []
            for handle, expected in zip(handles, identities):
                witness = self.api.exit_witness(handle)
                if witness is None:
                    return False  # liveness, not a failure or renewed observation lease
                if witness["pid"] != expected["pid"] or witness["creation_100ns"] != expected["creation_100ns"]:
                    raise Refusal("held exit identity changed")
                if witness["exit_100ns"] < witness["creation_100ns"] or witness["creation_100ns"] <= 0:
                    raise Refusal("invalid held terminal time")
                witnesses.append(witness)
            self.exit_proof = dict(owner_returncode=code, witnesses=witnesses,
                                   coverage="only previously held processes, not all writers")
            return True
        except Exception as error:
            self.custody_refusal = str(error)
            return False

    def finalize_observation(self):
        if self.observation is None or self.exit_proof is None or self.custody_refusal is not None or self.release_attempted:
            raise Refusal("observation custody is unresolved or already finalized")
        self.release_attempted = True
        first_error = None
        for resource in (self.monitor.chain, self.root):
            if resource is not None:
                try:
                    resource.close()
                except Exception as error:
                    if first_error is None:
                        first_error = error
        if first_error is not None:
            self.custody_refusal = str(first_error)
            raise Refusal("held query cleanup refused: " + self.custody_refusal)
        self.released = True

    def result(self):
        return copy.deepcopy(dict(observation=self.observation, first_observation_refusal=self.first_refusal,
                                  first_custody_refusal=self.custody_refusal, exit_proof=self.exit_proof,
                                  query_handles_released=self.released, release_attempted=self.release_attempted,
                                  execution_completion_authorized=False, all_writers_proven_stopped=False,
                                  durable_custody=False, selected_target_pass=False, admission=False))


class DispatchOnce:
    """A lost spawn ACK never licenses a second spawn, even if attach failed."""

    def __init__(self, attempt_id, directory):
        if re.fullmatch(r"[a-z][a-z0-9-]{0,63}", attempt_id) is None:
            raise Refusal("invalid local attempt identifier")
        self.attempt_id = attempt_id
        self.directory = Path(directory).resolve()
        self.attempted = False
        self.owner = self.custody = None
        self.acknowledged = False
        self.first_refusal = None

    def publish(self, name, document):
        # Exclusive markers deliberately survive uncertainty and process restart.
        # File fsync alone is not a cross-platform directory/power-loss proof.
        with (self.directory / name).open("xb") as output:
            output.write((json.dumps(document, sort_keys=True) + "\n").encode())
            output.flush()
            os.fsync(output.fileno())

    def start(self, spawn, attach, publish_ack):
        if self.attempted:
            raise Refusal("attempt already consumed; never respawn an uncertain launch")
        self.attempted = True
        try:
            self.publish("dispatch-consumed.json", dict(attempt_id=self.attempt_id, retry_allowed=False))
            self.owner = spawn()
            self.custody = attach(self.owner)
            self.publish("owner-attached.json", dict(attempt_id=self.attempt_id, pid=self.custody.root.pid,
                creation_100ns=self.custody.root.samples[0]["creation_100ns"]))
            publish_ack(self.attempt_id)
            self.acknowledged = True
        except Exception as error:
            self.first_refusal = str(error)
            if self.custody is not None:
                self.custody.freeze("dispatch acknowledgement refused: " + self.first_refusal)
        return self.result()

    def result(self):
        return dict(attempt_id=self.attempt_id, attempted=self.attempted, acknowledged=self.acknowledged,
                    first_dispatch_refusal=self.first_refusal, owner_object_retained=self.owner is not None,
                    attached_custody=self.custody is not None, retry_allowed=False,
                    durable_recovery=False, admission=False)
