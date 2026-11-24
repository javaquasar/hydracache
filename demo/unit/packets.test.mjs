import assert from "node:assert/strict";
import { describe, it } from "node:test";

import { packetFamily, packetFlights } from "../graph.js";

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
