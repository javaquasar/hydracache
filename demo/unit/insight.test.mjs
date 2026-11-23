// Unit tests for the snapshot view-model helpers (node:test, no browser).
import assert from "node:assert/strict";
import { describe, test } from "node:test";

import {
  describeAction,
  graphSummary,
  keyAgreement,
  leaderOf,
  nodeStatus,
  scriptAtStep,
  shortVersion,
  speedLabel,
  timelineEvents,
  timelineMarks,
  verdictView,
} from "../insight.js";

const baseSnapshot = {
  seed: 7,
  step: 40,
  intervention_count: 0,
  active_scenario: null,
  first_violation_step: 31,
  verdict: { status: "violated", invariant: "election_safety", detail: "two leaders" },
  nodes: [
    { id: "node-0", crashed: false, disabled: false, vote_state: "leader", term: 3 },
    { id: "node-1", crashed: false, disabled: false, vote_state: "follower", term: 3 },
    { id: "node-2", crashed: true, disabled: false, vote_state: "follower", term: 2 },
  ],
  links: [
    { from: "node-0", to: "node-1", state: "up" },
    { from: "node-1", to: "node-0", state: "partitioned" },
  ],
  keys: [],
  clients: [{ id: "client-a" }],
  subscribers: [],
  in_flight: [{}, {}],
};

describe("verdictView", () => {
  test("a plain-seed violation names the seed command and first step", () => {
    const view = verdictView(baseSnapshot);
    assert.equal(view.kind, "bad");
    assert.match(view.text, /vopr -- --seed 7 --steps 40/);
    assert.match(view.text, /first at step 31/);
  });

  test("an intervened or scenario violation never offers the plain seed command", () => {
    for (const snapshot of [
      { ...baseSnapshot, intervention_count: 2 },
      { ...baseSnapshot, active_scenario: "minority_partition_cannot_commit" },
    ]) {
      const view = verdictView(snapshot);
      assert.match(view.text, /--script hydracache-replay-seed-7-step-40\.json/);
      assert.doesNotMatch(view.text, /--seed 7/);
    }
  });

  test("a recovered run keeps its first violation visible", () => {
    const view = verdictView({ ...baseSnapshot, verdict: { status: "holding" } });
    assert.equal(view.kind, "warn");
    assert.match(view.text, /first reported at step 31/);
  });

  test("a clean run is ok", () => {
    const view = verdictView({ ...baseSnapshot, verdict: { status: "holding" }, first_violation_step: null });
    assert.equal(view.kind, "ok");
  });
});

describe("keyAgreement", () => {
  const agreement = keyAgreement({
    ...baseSnapshot,
    keys: [
      { key: "a", replicas: [{ node_id: "node-0", version: 5 }, { node_id: "node-1", version: 5 }] },
      { key: "b", replicas: [{ node_id: "node-0", version: 5 }, { node_id: "node-1", version: 6 }] },
      { key: "c", replicas: [{ node_id: "node-0", version: 5 }] },
    ],
  });

  test("ignores down nodes when judging agreement", () => {
    assert.equal(agreement.keys[0].agree, true);
    assert.equal(agreement.keys[0].cells.find((cell) => cell.node === "node-2").live, false);
  });

  test("flags different checksums and missing live replicas", () => {
    assert.equal(agreement.keys[1].agree, false);
    assert.equal(agreement.keys[2].agree, false);
    assert.equal(agreement.keys[2].cells.find((cell) => cell.node === "node-1").diverged, true);
    assert.equal(agreement.agree, false);
  });
});

describe("timeline helpers", () => {
  test("rewinding keeps interventions at or before the target step", () => {
    const script = scriptAtStep(
      { version: 1, seed: 7, actions: [{ type: "crash", at_step: 3 }, { type: "restart", at_step: 9 }] },
      5,
    );
    assert.equal(script.target_step, 5);
    assert.deepEqual(script.actions.map((action) => action.type), ["crash"]);
  });

  test("events merge recorded actions with observed leader and verdict changes", () => {
    const events = timelineEvents(
      { actions: [{ type: "crash", node: "node-0", at_step: 4 }] },
      new Map([
        [3, { leader: "node-0", verdict: "holding" }],
        [6, { leader: "node-1", verdict: "holding" }],
        [8, { leader: "node-1", verdict: "violated", invariant: "election_safety" }],
      ]),
    );
    assert.deepEqual(
      events.map((event) => `${event.step}:${event.kind}:${event.text}`),
      ["4:action:crash node-0", "6:leader:leader is node-1", "8:verdict:violation: election_safety"],
    );
  });

  test("every control action type has a readable description", () => {
    for (const type of [
      "crash", "restart", "isolate", "rejoin", "disable", "enable", "add_node", "partition",
      "heal", "drop_next", "delay_next", "workload", "subscribe", "push_event", "mode_change", "step",
    ]) {
      const fields = { node: "n", from: "a", to: "b", n: 2, mode: "mixed", millis: 250, client: "c", ns: "ns", key: "k", enabled: false };
      const text = describeAction({ type, ...fields });
      assert.notEqual(text, type, `${type} falls through to the raw type`);
      assert.doesNotMatch(text, /undefined/, type);
    }
  });
});

describe("node and graph text", () => {
  test("crash and disable override the election state", () => {
    assert.equal(nodeStatus({ crashed: true, vote_state: "leader" }), "crashed");
    assert.equal(nodeStatus({ disabled: true, vote_state: "leader" }), "disabled");
    assert.equal(nodeStatus({ vote_state: "candidate" }), "candidate");
    assert.equal(leaderOf(baseSnapshot), "node-0");
  });

  test("the graph summary states nodes, degraded links and traffic", () => {
    assert.equal(
      graphSummary(baseSnapshot),
      "3 nodes: node-0 leader, node-1 follower, node-2 crashed. Directed links: 1 partitioned. 1 client(s), 0 subscriber(s), 2 message(s) in flight.",
    );
  });

  test("checksums render as a short stable suffix", () => {
    assert.equal(shortVersion(null), "—");
    assert.equal(shortVersion(255), "0000ff");
    assert.equal(shortVersion(-1), "ffffff");
  });
});

describe("speedLabel", () => {
  test("shows seconds at or above one second and milliseconds below", () => {
    assert.equal(speedLabel(1200), "1.2 s/step");
    assert.equal(speedLabel(1000), "1.0 s/step");
    assert.equal(speedLabel(80), "80 ms/step");
  });

  test("rejects non-positive or non-numeric intervals", () => {
    assert.equal(speedLabel(0), "—");
    assert.equal(speedLabel("fast"), "—");
  });
});

describe("timelineMarks", () => {
  const events = [
    { step: 2, kind: "action", text: "crash node-1" },
    { step: 2, kind: "leader", text: "leader is node-0" },
    { step: 5, kind: "verdict", text: "violation: read-your-writes" },
    { step: 12, kind: "action", text: "heal" },
  ];

  test("places events proportionally over the scrubber range", () => {
    const marks = timelineMarks(events, 0, 10);
    assert.deepEqual(
      marks.map((mark) => [mark.step, mark.percent]),
      [
        [2, 20],
        [5, 50],
      ],
    );
  });

  test("merges events on one step and keeps the most important kind", () => {
    const [first] = timelineMarks(events, 0, 10);
    assert.equal(first.kind, "leader");
    assert.equal(first.text, "step 2: crash node-1; leader is node-0");
  });

  test("returns no marks for an empty range", () => {
    assert.deepEqual(timelineMarks(events, 4, 4), []);
    assert.deepEqual(timelineMarks(null, 0, 10), []);
  });
});
