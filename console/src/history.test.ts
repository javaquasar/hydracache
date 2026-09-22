import { describe, expect, it } from "vitest";
import { HISTORY_LIMITS, SERIES_DEFINITIONS, SnapshotHistory, finiteNumber, shouldPauseCollection } from "./history";

function sample(epoch: number, entries = 5, hits = 8): Record<string, unknown> {
  return {
    authority_epoch: epoch,
    replication: { success_total: 10, failure_total: 2, backpressure_total: 1, under_replicated: 0, repair_debt: 0, zone_underspread: 0 },
    reshard: { moves_inflight: 0, backfill_lag: 0 },
    cache: { entries, hits_total: hits, misses_total: 2, loads_total: 2, admission_queue_depth: 0, admission_rejected_total: 0 },
    consensus: { apply_lag: 0 },
    members: [{ node: "member-a" }],
  };
}

describe("SnapshotHistory", () => {
  it("derives counters, preserves gauges and turns resets into gaps", () => {
    expect(SERIES_DEFINITIONS.find(([name]) => name === "cache.entries")?.[2]).toBe("gauge");
    const history = new SnapshotHistory();
    history.ingest(sample(1), 1);
    history.ingest(sample(1, 7, 11), 2);
    expect(history.points("cache.entries").at(-1)?.value).toBe(7);
    expect(history.points("cache.hits").at(-1)?.value).toBe(3);
    history.ingest(sample(1, 3, 1), 3);
    expect(history.points("cache.hits").at(-1)).toMatchObject({ value: null, reset: true });
    expect(history.points("cache.entries").at(-1)?.value).toBe(3);
  });

  it("keeps missing and non-finite values as gaps", () => {
    expect(finiteNumber(undefined)).toBeNull();
    expect(finiteNumber(Number.NaN)).toBeNull();
    expect(finiteNumber(Number.POSITIVE_INFINITY)).toBeNull();
  });

  it("clears incompatible epochs and bounds member identity", () => {
    const history = new SnapshotHistory();
    history.ingest({ ...sample(1), members: [{ node: "z" }, { node: "a" }] }, 1);
    history.ingest(sample(2), 2);
    expect(history.snapshot().epoch).toBe(2);
    expect(history.points("cache.entries")).toHaveLength(1);
    expect(history.snapshot().memberSet).toEqual(["member-a"]);
  });

  it("evicts oldest samples under point and byte budgets", () => {
    const history = new SnapshotHistory({ limits: { maxPointsPerSeries: 3, maxTotalPoints: 20, maxBytes: 1_700 } });
    for (let index = 0; index < 40; index += 1) history.ingest(sample(1, index, index), index);
    expect(history.snapshot().totalPoints).toBeLessThanOrEqual(20);
    expect(history.snapshot().byteSize).toBeLessThanOrEqual(1_700);
    expect(history.points("cache.entries").length).toBeLessThanOrEqual(3);
    expect(history.snapshot().byteSize).toBe(new TextEncoder().encode(JSON.stringify([...history.series])).byteLength);
  });

  it("survives an accelerated twelve-hour run within production budgets", () => {
    const history = new SnapshotHistory();
    for (let minute = 0; minute < 720; minute += 1) history.ingest(sample(7, minute % 101, minute * 3), minute * 60_000);
    expect(history.snapshot().seriesCount).toBeLessThanOrEqual(HISTORY_LIMITS.maxSeries);
    expect(history.snapshot().totalPoints).toBeLessThanOrEqual(HISTORY_LIMITS.maxTotalPoints);
    expect(history.snapshot().byteSize).toBeLessThanOrEqual(HISTORY_LIMITS.maxBytes);
  });
});

export function canaryCollectionPausesForHiddenOrOfflineTabs(): void {
  if (process.env.HYDRACACHE_CANARY_DEFECT === "MC72-W4") {
    throw new Error("HC-CANARY-RED:MC72-W4 browser history collection continued while hidden or offline");
  }
  expect(shouldPauseCollection(true, true)).toBe(true);
  expect(shouldPauseCollection(false, false)).toBe(true);
  expect(shouldPauseCollection(false, true)).toBe(false);
}

it("collection pauses while hidden or offline", () => {
  canaryCollectionPausesForHiddenOrOfflineTabs();
});
