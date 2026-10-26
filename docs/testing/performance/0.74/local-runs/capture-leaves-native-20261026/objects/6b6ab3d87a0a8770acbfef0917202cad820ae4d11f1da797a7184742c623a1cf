"""Caller-owned observation state machine; never launches or kills a process.

Absence during discovery is WAITING, not an attachment failure. Once refused,
no further census, attachment or sampling is attempted; the owner still drains
under its own original deadline. Outer expiry retains live-owner custody.
"""

import math

from owned_process_observer import Refusal
from process_lineage import HeldLineage, candidates


class Coordinator:
    DISCOVERY_SECONDS = 30
    DISCOVERY_SNAPSHOTS = 300
    OUTER_SECONDS = 1200
    SAMPLE_SECONDS = 1

    def __init__(self, owner, root, api, policies):
        self.owner, self.root, self.api, self.policies = owner, root, api, policies
        self.chain = None
        self.first_refusal = None
        self.events = []
        self.last_elapsed = -1
        self.discovery_snapshots = 0
        self.next_sample = None
        self.leaf_terminal = False
        self.owner_exit_observed = False
        self.owner_returncode = None
        self.outer_expired = False
        self.closed = False

    def refuse(self, error):
        if self.first_refusal is None:
            self.first_refusal = str(error)
            self.events.append(dict(kind="first-refusal", reason=self.first_refusal))

    def sample(self):
        if self.leaf_terminal:
            return
        row = self.chain.sample()
        self.leaf_terminal = row["terminal"]
        if self.leaf_terminal:
            self.events.append(dict(kind="leaf-terminal"))

    def tick(self, elapsed):
        """Elapsed is from the caller's pre-launch origin, never a renewed lease."""
        if self.closed or self.owner_exit_observed or self.outer_expired:
            raise Refusal("coordinator no longer accepts ticks")
        if isinstance(elapsed, bool) or not isinstance(elapsed, (int, float)) or not math.isfinite(elapsed) or elapsed < self.last_elapsed or elapsed < 0:
            self.refuse("invalid or regressing observation clock")
            raise Refusal(self.first_refusal)
        self.last_elapsed = elapsed
        code = self.owner.poll()
        if code is not None:
            self.owner_exit_observed, self.owner_returncode = True, code
            if self.first_refusal is None:
                try:
                    if self.chain is None:
                        raise Refusal("owner exited before leaf attachment")
                    self.sample()
                    if not self.leaf_terminal:
                        raise Refusal("leaf still live after owner exit")
                    self.root.sample()
                except Exception as error:
                    self.refuse(error)
            self.events.append(dict(kind="owner-exit", code=code))
            return
        if elapsed >= self.OUTER_SECONDS:
            self.refuse("outer observation bound; live owner custody retained")
            self.outer_expired = True
            return
        if self.first_refusal is not None:
            return  # no observation retry, no early stop of the original owner
        try:
            if self.chain is None:
                if elapsed >= self.DISCOVERY_SECONDS or self.discovery_snapshots >= self.DISCOVERY_SNAPSHOTS:
                    raise Refusal("bounded discovery exhausted")
                self.discovery_snapshots += 1
                selected = candidates(self.api.snapshot(), self.root.pid, self.policies)
                if not selected:
                    return
                # One attachment attempt only. HeldLineage independently takes
                # two snapshots and checks held identities around every edge.
                self.chain = HeldLineage(self.root, self.api, self.policies)
                self.leaf_terminal = self.chain.samples[-1]["terminal"]
                self.next_sample = elapsed + self.SAMPLE_SECONDS
                self.events.append(dict(kind="attached", pids=list(self.chain.pids)))
                if self.leaf_terminal:
                    self.events.append(dict(kind="leaf-terminal"))
            elif not self.leaf_terminal and elapsed >= self.next_sample:
                self.sample()
                # Missed observer slots are not synthesized or caught up in a burst.
                missed = int((elapsed - self.next_sample) // self.SAMPLE_SECONDS)
                self.next_sample += (missed + 1) * self.SAMPLE_SECONDS
        except Exception as error:
            self.refuse(error)

    def result(self):
        return dict(schema=1, first_refusal=self.first_refusal, events=list(self.events),
                    discovery_snapshots=self.discovery_snapshots,
                    owner_exit_observed=self.owner_exit_observed,
                    owner_returncode=self.owner_returncode,
                    live_owner_custody_retained=not self.owner_exit_observed,
                    leaf_terminal=self.leaf_terminal, outer_expired=self.outer_expired,
                    observations_complete=self.owner_exit_observed and self.leaf_terminal and self.first_refusal is None,
                    identities=self.chain.identities if self.chain else [],
                    samples=self.chain.samples if self.chain else [],
                    selected_target_pass=False, full_package_pass=False, admission=False)

    def close(self):
        # This is observation custody only, not a product writer fence. The
        # caller still owns the original root observer and Popen lifetime.
        if not self.owner_exit_observed:
            raise Refusal("live owner: caller must retain custody")
        if not self.closed:
            self.closed = True
            if self.chain is not None:
                self.chain.close()
