import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { foldRoutineTraffic, packetFamily, packetFlights, tidyTargets } from "../graph.js";

const nodes = [
  { id: "node-0", up: true, crashed: false, disabled: false },
  { id: "node-1", up: true, crashed: false, disabled: false },
  { id: "node-2", up: true, crashed: false, disabled: false },
];

function snapshot({ step = 10, inFlight, nodeOverrides = {}, partitioned = [] }) {
  return {
    step,
    logical_time_millis: step,
    nodes: nodes.map((node) => ({ ...node, ...(nodeOverrides[node.id] ?? {}) })),
    links: [
      ["node-0", "node-1"],
      ["node-1", "node-0"],
      ["node-0", "node-2"],
      ["node-2", "node-0"],
    ].map(([from, to]) => ({ from, to, state: partitioned.includes(`${from}>${to}`) ? "partitioned" : "up" })),
    in_flight: inFlight,
  };
}

const message = (id, from, to, remaining = 0, kind = "heartbeat") => ({
  id,
  from,
  to,
  kind,
  remaining_millis: remaining,
});

describe("packetFlights", () => {
  it("an undelayed message crosses the whole edge in one step", () => {
    const { flights } = packetFlights(snapshot({ inFlight: [message("a", "node-0", "node-1")] }));
    assert.equal(flights[0].start, 0);
    assert.equal(flights[0].end, 1);
    assert.equal(flights[0].doomed, false);
  });

  it("a delayed message covers one step's share and keeps its flight length", () => {
    const first = packetFlights(snapshot({ inFlight: [message("d", "node-0", "node-2", 3)] }));
    assert.deepEqual([first.flights[0].start, first.flights[0].end], [0, 0.25]);
    const second = packetFlights(
      snapshot({ step: 11, inFlight: [message("d", "node-0", "node-2", 2)] }),
      first.totals,
    );
    assert.deepEqual([second.flights[0].start, second.flights[0].end], [0.25, 0.5]);
    const last = packetFlights(snapshot({ step: 13, inFlight: [message("d", "node-0", "node-2", 0)] }), second.totals);
    assert.deepEqual([last.flights[0].start, last.flights[0].end], [0.75, 1]);
  });

  it("converts logical milliseconds to steps", () => {
    const { flights } = packetFlights({
      ...snapshot({ inFlight: [message("m", "node-0", "node-1", 30)] }),
      step: 10,
      logical_time_millis: 100,
    });
    assert.deepEqual([flights[0].start, flights[0].end], [0, 0.25]);
  });

  it("marks a message lost when its directed link is cut or its target is down", () => {
    const { flights } = packetFlights(
      snapshot({
        inFlight: [
          message("cut", "node-0", "node-1"),
          message("reverse", "node-1", "node-0"),
          message("crashed", "node-0", "node-2"),
          message("from-crashed", "node-2", "node-0"),
        ],
        partitioned: ["node-0>node-1"],
        nodeOverrides: { "node-2": { up: false, crashed: true } },
      }),
    );
    const doomed = Object.fromEntries(flights.map((flight) => [flight.message.id, flight.doomed]));
    assert.deepEqual(doomed, { cut: true, reverse: false, crashed: true, "from-crashed": false });
  });

  it("gives messages on the same edge separate lanes, in either direction", () => {
    const { flights } = packetFlights(
      snapshot({
        inFlight: [
          message("a", "node-0", "node-1"),
          message("b", "node-1", "node-0"),
          message("c", "node-0", "node-2"),
        ],
      }),
    );
    assert.deepEqual(
      flights.map((flight) => flight.lane),
      [0, 1, 0],
    );
  });
});

describe("packetFamily", () => {
  it("groups engine message kinds into votes, data, events and control", () => {
    const families = Object.fromEntries(
      [
        "vote_request",
        "vote_response",
        "replicate_put",
        "upserted",
        "replicate_invalidate",
        "replicate_flush",
        "invalidated",
        "leader_heartbeat",
        "heartbeat_ack",
        "heartbeat",
        "ack",
        "something_else",
      ].map((kind) => [kind, packetFamily(kind)]),
    );
    assert.deepEqual(families, {
      vote_request: "vote",
      vote_response: "vote",
      replicate_put: "data",
      upserted: "data",
      replicate_invalidate: "event",
      replicate_flush: "event",
      invalidated: "event",
      leader_heartbeat: "control",
      heartbeat_ack: "control",
      heartbeat: "control",
      ack: "control",
      something_else: "message",
    });
  });
});

describe("foldRoutineTraffic", () => {
  const flight = (from, to, kind, extra = {}) => ({ message: { from, to, kind }, start: 0, end: 1, doomed: false, ...extra });

  it("keeps one routine dot per direction on a busy edge and counts the rest", () => {
    const flights = [
      flight("a", "b", "leader_heartbeat"),
      flight("a", "b", "heartbeat"),
      flight("b", "a", "heartbeat_ack"),
      flight("b", "a", "ack"),
      flight("a", "b", "vote_request"),
    ];
    const { shown, folded } = foldRoutineTraffic(flights);
    assert.deepEqual(
      shown.map((item) => item.message.kind),
      ["leader_heartbeat", "heartbeat_ack", "vote_request"],
    );
    assert.deepEqual([...folded], [["a|b", 2]]);
  });

  it("never folds delayed, doomed or quiet-edge messages", () => {
    const flights = [
      flight("a", "b", "heartbeat"),
      flight("a", "b", "heartbeat", { end: 0.25 }),
      flight("a", "b", "ack", { doomed: true }),
      flight("a", "b", "ack"),
    ];
    assert.equal(foldRoutineTraffic(flights).shown.length, 3);
    assert.equal(foldRoutineTraffic(flights.slice(0, 3)).folded.size, 0);
  });
});

describe("tidyTargets", () => {
  it("places nodes on a ring from the top and entities outside their node", () => {
    const nodes = [{ id: "node-1" }, { id: "node-0" }, { id: "node-2" }];
    const targets = tidyTargets(nodes, [{ id: "client-a", connected_node: "node-0" }], [{ id: "r@ns", connected_node: "node-0" }]);
    const top = targets.get("node-0");
    assert.ok(top.y < targets.get("node-1").y && top.y < targets.get("node-2").y, "node-0 is at the top");
    assert.equal(Math.round(top.x), 400);
    for (const id of ["client:client-a", "sub:r@ns"]) {
      assert.ok(targets.get(id).y < top.y, `${id} sits outside its node`);
    }
    assert.notDeepEqual(targets.get("client:client-a"), targets.get("sub:r@ns"), "fanned out");
    // The same snapshot gives the same picture.
    assert.deepEqual(tidyTargets(nodes), tidyTargets([...nodes].reverse()));
  });
});
