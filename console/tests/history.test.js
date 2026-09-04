import assert from "node:assert/strict";
import test from "node:test";

import {
  HISTORY_LIMITS,
  SERIES_DEFINITIONS,
  SnapshotHistory,
  finiteNumber,
  shouldPauseCollection,
} from "../history.js";

function sample(epoch, overrides = {}) {
  return {
    authority_epoch: epoch,
    replication: {
      success_total: 10,
      failure_total: 2,
      backpressure_total: 1,
      under_replicated: 0,
      repair_debt: 0,
      zone_underspread: 0,
    },
    reshard: { moves_inflight: 0, backfill_lag: 0 },
    cache: {
      entries: 5,
      hits_total: 8,
      misses_total: 2,
      loads_total: 2,
      admission_queue_depth: 0,
      admission_rejected_total: 0,
    },
    consensus: { apply_lag: 0 },
    members: [{ node: "member-a" }],
    ...overrides,
  };
}

test("metric kinds keep gauges and derive counter deltas", () => {
  assert.equal(SERIES_DEFINITIONS.find(([name]) => name === "cache.entries")[2], "gauge");
  assert.equal(SERIES_DEFINITIONS.find(([name]) => name === "cache.hits")[2], "counter");
  const history = new SnapshotHistory();
  history.ingest(sample(1), 1);
  history.ingest(sample(1, { cache: { ...sample(1).cache, entries: 7, hits_total: 11 } }), 2);
  assert.equal(history.points("cache.entries").at(-1).value, 7);
  assert.equal(history.points("cache.hits").at(-1).value, 3);
});

test("counter reset is a gap while gauges are never differentiated", () => {
  const history = new SnapshotHistory();
  history.ingest(sample(1), 1);
  history.ingest(sample(1, { cache: { ...sample(1).cache, entries: 3, hits_total: 1 } }), 2);
  assert.deepEqual(
    { value: history.points("cache.hits").at(-1).value, reset: history.points("cache.hits").at(-1).reset },
    { value: null, reset: true },
  );
  assert.equal(history.points("cache.entries").at(-1).value, 3);
});

test("missing NaN and infinity remain gaps rather than fake zeroes", () => {
  assert.equal(finiteNumber(undefined), null);
  assert.equal(finiteNumber(Number.NaN), null);
  assert.equal(finiteNumber(Number.POSITIVE_INFINITY), null);
  const history = new SnapshotHistory();
  history.ingest(sample(1, { consensus: { apply_lag: Number.NaN } }), 1);
  assert.equal(history.points("consensus.apply_lag")[0].missing, true);
  assert.equal(history.points("consensus.apply_lag")[0].value, null);
});

test("authority epoch change clears incompatible history", () => {
  const history = new SnapshotHistory();
  history.ingest(sample(1), 1);
  history.ingest(sample(1), 2);
  history.ingest(sample(2), 3);
  assert.equal(history.snapshot().epoch, 2);
  assert.equal(history.points("cache.entries").length, 1);
  assert.equal(history.points("cache.hits")[0].value, null);
});

test("member set changes are captured as bounded sorted evidence", () => {
  const history = new SnapshotHistory();
  history.ingest(sample(1, { members: [{ node: "z" }, { node: "a" }] }), 1);
  assert.deepEqual(history.snapshot().memberSet, ["a", "z"]);
  history.ingest(sample(1, { members: [{ node: "b" }] }), 2);
  assert.deepEqual(history.snapshot().memberSet, ["b"]);
});

function canaryCollectionPausesForHiddenOrOfflineTabs() {
  const mutant = process.env.HYDRACACHE_CANARY_DEFECT === "MC72-W4";
  if (!shouldPauseCollection(true, true, { disableHiddenPause: mutant })) {
    throw new Error("HC-CANARY-RED:MC72-W4 hidden tab continued collecting history");
  }
  assert.equal(shouldPauseCollection(false, false), true);
  assert.equal(shouldPauseCollection(false, true), false);
}

test("collection pauses for hidden or offline tabs", canaryCollectionPausesForHiddenOrOfflineTabs);

test("ring evicts oldest points by per-series point and byte budgets", () => {
  const history = new SnapshotHistory({
    limits: { maxPointsPerSeries: 3, maxTotalPoints: 20, maxBytes: 1_700 },
    disableEviction: process.env.HYDRACACHE_CANARY_DEFECT === "MC72-W4",
  });
  for (let index = 0; index < 40; index += 1) {
    history.ingest(sample(1, { cache: { ...sample(1).cache, entries: index, hits_total: index } }), index);
  }
  const current = history.snapshot();
  if (process.env.HYDRACACHE_CANARY_DEFECT === "MC72-W4" && current.totalPoints > 20) {
    throw new Error("HC-CANARY-RED:MC72-W4 browser history exceeded frozen budget");
  }
  assert.ok(current.totalPoints <= 20);
  assert.ok(current.byteSize <= 1_700);
  assert.ok(history.points("cache.entries").length <= 3);
});

test("accelerated multi-hour soak remains within production budgets", () => {
  const history = new SnapshotHistory();
  for (let minute = 0; minute < 12 * 60; minute += 1) {
    history.ingest(
      sample(7, {
        cache: { ...sample(7).cache, entries: minute % 101, hits_total: minute * 3 },
        members: Array.from({ length: minute % 31 }, (_, index) => ({ node: `member-${index}` })),
      }),
      minute * 60_000,
    );
  }
  const current = history.snapshot();
  assert.ok(current.seriesCount <= HISTORY_LIMITS.maxSeries);
  assert.ok(current.totalPoints <= HISTORY_LIMITS.maxTotalPoints);
  assert.ok(current.byteSize <= HISTORY_LIMITS.maxBytes);
});
