# 002 - Raft Snapshot Bugs, AI Agents, and the Cost of Ignoring Contradictions

![LinkedIn article cover image](002-raft-snapshot-agent-bug-cover.jpg)

This note captures and reflects on Andrii Rodionov's article
["Can an AI Agent Fix a Four-Year-Old Raft Snapshot Replication Bug?"](https://www.linkedin.com/pulse/can-ai-agent-fix-four-year-old-raft-snapshot-bug-andrii-rodionov-hdpue/),
published on July 2, 2026.

It is a comprehensive technical digest and HydraCache planning note, not a verbatim copy of the
LinkedIn article. The goal is to preserve the full engineering material: the scenario, root cause,
agent-debugging failure mode, source links, and concrete test ideas for HydraCache.

## Source Links

- Original LinkedIn article:
  <https://www.linkedin.com/pulse/can-ai-agent-fix-four-year-old-raft-snapshot-bug-andrii-rodionov-hdpue/>
- Author profile linked by LinkedIn:
  <https://de.linkedin.com/in/andriirodionov>
- GitHub issue discussed in the article:
  <https://github.com/hazelcast/hazelcast/issues/21438>
- GitHub pull request referenced by the issue:
  <https://github.com/hazelcast/hazelcast/pull/22793>
- Background link embedded in the article's snapshot explanation:
  <https://dev.to/justlorain/how-to-build-your-own-distributed-kv-storage-system-using-the-etcd-raft-library-2-398c>
- Hazelcast CP subsystem management reference:
  <https://docs.hazelcast.com/hazelcast/5.7/cp-subsystem/management>
- Original LinkedIn cover image URL:
  <https://media.licdn.com/dms/image/v2/D4E12AQHsksh72YqDdw/article-cover_image-shrink_720_1280/B4EZ8jy3gRK4AQ-/0/1783011965993?e=2147483647&v=beta&t=50WlHnr5BeeHRdzVbOWjlRk60vG_tCE0lb7sHOdE4sw>
- Local static cover image:
  [`002-raft-snapshot-agent-bug-cover.jpg`](002-raft-snapshot-agent-bug-cover.jpg)
- HydraCache follow-on plan:
  [`V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md`](../plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md)

## Article Metadata

- Title: "Can an AI Agent Fix a Four-Year-Old Raft Snapshot Replication Bug?"
- Author: Andrii Rodionov
- Published: July 2, 2026
- System under discussion: Hazelcast CP subsystem, Raft snapshot restore, CP membership add/remove.
- Experiment subject: Claude Opus 4.8 agent investigating a known Hazelcast bug after the real fix
  already existed.

## The Bug Scenario

The article studies a flaky Hazelcast test:
`CPMemberAddRemoveTest.when_snapshotIsTakenWhileRemovingCPLeader_newMemberInstallsSnapshot`.

The rough sequence is:

1. A Raft leader node is shut down.
2. A new leader election happens.
3. A fresh member is added to the CP subsystem.
4. The fresh member is too far behind for ordinary log replay.
5. The leader sends a Raft snapshot so the new member can catch up.
6. The snapshot was taken in the middle of a membership change.
7. The new member must install the snapshot and then apply the remaining log tail.

The expected result is convergence: the new member's local `activeMembers` state should eventually
match the authoritative committed CP member list returned by a linearizable query.

The observed failure shape is more interesting than a simple mismatch. The new member can keep the
removed former leader in its local view and fail to include itself. In the GitHub issue, the expected
view contains the members on ports `5702`, `5703`, and `5704`, while the actual view contains `5702`,
`5701`, and `5703`.

This matters because it is not only stale display data. It means the restored member cannot apply the
membership tail after snapshot installation, so the local state machine freezes at an old membership
boundary.

## Snapshot Mechanics Under Test

The test exercises the core purpose of Raft snapshots:

- the log grows as committed operations accumulate;
- an old or new follower may not have the entries needed to replay from the beginning;
- the leader can ship a compacted state-machine snapshot at a commit index;
- the follower installs the snapshot and then applies entries after that index.

The hard part is not ordinary snapshot restore. The hard part is a snapshot taken during a
membership transition. That means snapshot state and the remaining log tail must compose correctly.
If either side has hidden mutation, stale references, or mismatched commit indexes, the follower can
install a state that later rejects the very operations required to converge.

## Root Cause

The article's root cause is an immutability violation.

The snapshot was supposed to be a stable point-in-time state. Instead, it exposed `CPGroupInfo`
entities without deep-copying them. On restore, the same instances were placed into the live group
map. Later membership-change operations mutated those shared instances in place.

That mutation corrupted the retained snapshot itself. Its membership commit index no longer matched
the pending membership-change schedule. When a new member installed that corrupted snapshot, the
post-snapshot add/remove operations failed their consistency check and could not be applied.

The practical lesson is blunt: a snapshot cannot share mutable state with the live state machine. If
it does, the snapshot is not a snapshot. It is a delayed alias.

## Why The Bug Was Flaky

The failure needs several timing-sensitive conditions to align:

- snapshot timing: the snapshot must be captured during a membership change;
- election timing: a specific leader/follower arrangement must ship and install the snapshot;
- mutation timing: the shared object must be mutated before the retained snapshot is reused.

This is why repeated test runs may look mostly green while the underlying bug remains real.

The article reports one agent attempt that reproduced the failure only once in 215 runs. The problem
is that rare reproduction is still evidence when the failing trace points at a state-machine
contradiction. Treating rarity as irrelevance is the exact wrong move for this class of bug.

## AI Agent Experiment

The author tried three agent investigations.

In the first attempt, the agent tried to reproduce the issue. After a rare reproduction, it moved
toward dismissing the failure as environmental noise and suggested speculative changes. The most
concerning part was that it downplayed the important Raft operation error rather than using it as the
main contradiction signal.

In the second and third attempts, the agent was asked to reason from code and logs. It still anchored
on incorrect hypotheses and failed to converge on the mutability bug. After the true root cause was
revealed, the agent's post-mortem admitted that it had trusted a wrong intermediate assumption and
then tried to fit the evidence around it.

Short compliant excerpts from the article:

- "Could not apply add-member ... and remove-member ..."
- "chain of correct sub-claims leading to a wrong conclusion"

The useful point is not "do not use AI." The useful point is that autonomous debugging needs a
protocol. Without one, an agent can produce plausible explanations while quietly discarding the best
evidence.

## Engineering Lessons

### 1. Reproduction frequency is not severity

A flaky test that fails once in hundreds of runs can still expose a correctness bug. Distributed
systems often fail only when schedule, election, compaction, persistence, and mutation ordering line
up. Rarity is normal for this bug class.

### 2. Error messages from state-machine apply are evidence

If a membership operation cannot apply after snapshot restore, the error is not "noise" until proven
otherwise. It is a contradiction between the restored snapshot and the committed log tail.

### 3. Snapshot state must be owned or immutable

Snapshots should be byte-owned, deep-copied, copy-on-write, or otherwise protected from live-state
mutation. Reusing live objects inside snapshot exports is a trap.

### 4. Mid-transition snapshots need first-class tests

It is not enough to test clean snapshot restore. Systems need tests where snapshot capture happens
between remove/add, before/after conf-state persistence, and before tail operations that must still
apply.

### 5. Agents need a contradiction ledger

For flaky distributed bugs, an AI assistant should keep an explicit list of signals that contradict
the current hypothesis. It should not be allowed to close an investigation with "probably CI" while a
state-machine error remains unexplained.

## Reflection For HydraCache

HydraCache already used `0.62.0` and `0.62.1` to build the first raft/gossip/failpoint proof layer:

- deterministic raft message filters;
- gossip fault filters;
- real-process daemon cluster tests;
- membership-history checks;
- wire/id properties;
- golden vectors;
- snapshot crash failpoint coverage.

The article shows the next hardening step: not only crash windows, but snapshot aliasing and
mid-membership snapshot composition.

For HydraCache, the test expansion should check:

- exported raft metadata snapshots do not share mutable state with live membership state;
- a snapshot captured during remove/add voter activity can be restored and followed by log-tail
  replay to convergence;
- a restored node never accepts a snapshot whose membership commit indexes and command schedule are
  internally inconsistent;
- apply errors after snapshot restore are captured as release-blocking evidence, not downgraded;
- canaries can intentionally reintroduce aliasing or tail-apply skipping and turn the guard tests red.

## Direct Translation Into 0.64

The follow-on release plan is
[`0.64.0 Raft Snapshot & Agentic Debugging Test Expansion`](../plans/V0_64_RAFT_SNAPSHOT_AND_AGENTIC_DEBUGGING_TEST_EXPANSION_PLAN.md).

Its core thesis:

```text
0.62 proved raft/gossip/failpoint harnesses exist.
0.62.1 closed the first proof cleanup.
0.64 expands the test matrix around the class of bug this article describes:
snapshot aliasing, mid-membership snapshot restore, committed-tail convergence,
and agent/debugging guardrails for flaky distributed failures.
```

The release should remain test-first and feature-light. If it discovers a production bug, fix the bug
narrowly, but the release theme is evidence quality, not new surface area.
