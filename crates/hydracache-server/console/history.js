export const HISTORY_LIMITS = Object.freeze({
  maxSeries: 24,
  maxPointsPerSeries: 360,
  maxTotalPoints: 4_320,
  maxBytes: 262_144,
});

export const SERIES_DEFINITIONS = Object.freeze([
  ["replication.success", "replication.success_total", "counter"],
  ["replication.failure", "replication.failure_total", "counter"],
  ["replication.backpressure", "replication.backpressure_total", "counter"],
  ["replication.under_replicated", "replication.under_replicated", "gauge"],
  ["replication.repair_debt", "replication.repair_debt", "gauge"],
  ["replication.zone_underspread", "replication.zone_underspread", "gauge"],
  ["reshard.moves", "reshard.moves_inflight", "gauge"],
  ["reshard.backfill_lag", "reshard.backfill_lag", "gauge"],
  ["cache.entries", "cache.entries", "gauge"],
  ["cache.hits", "cache.hits_total", "counter"],
  ["cache.misses", "cache.misses_total", "counter"],
  ["cache.loads", "cache.loads_total", "counter"],
  ["cache.admission_queue", "cache.admission_queue_depth", "gauge"],
  ["cache.admission_rejected", "cache.admission_rejected_total", "counter"],
  ["consensus.apply_lag", "consensus.apply_lag", "gauge"],
]);

export class SnapshotHistory {
  constructor(options = {}) {
    this.limits = Object.freeze({ ...HISTORY_LIMITS, ...(options.limits ?? {}) });
    this.disableEviction = options.disableEviction === true;
    this.epoch = null;
    this.startedAt = null;
    this.lastMemberSet = [];
    this.series = new Map();
    this.byteSize = 0;
    this.totalPoints = 0;
  }

  ingest(snapshot, capturedAt = Date.now()) {
    const epoch = finiteNumber(snapshot?.authority_epoch ?? snapshot?.cluster?.authority_epoch);
    if (this.epoch !== null && epoch !== this.epoch) {
      this.clear();
    }
    this.epoch = epoch;
    this.startedAt ??= capturedAt;
    this.lastMemberSet = boundedMemberSet(snapshot?.members);

    for (const [name, path, kind] of SERIES_DEFINITIONS) {
      if (!this.series.has(name) && this.series.size >= this.limits.maxSeries) {
        continue;
      }
      const raw = finiteNumber(readPath(snapshot, path));
      const values = this.series.get(name) ?? [];
      const previous = values.at(-1);
      let value = raw;
      let reset = false;
      if (kind === "counter") {
        if (raw === null || previous?.raw == null) {
          value = null;
        } else if (raw < previous.raw) {
          value = null;
          reset = true;
        } else {
          value = raw - previous.raw;
        }
      }
      values.push({ at: capturedAt, value, raw, missing: raw === null, reset });
      this.series.set(name, values);
      this.totalPoints += 1;
    }
    this.recountBytes();
    if (!this.disableEviction) {
      this.evict();
    }
    return this.snapshot();
  }

  clear() {
    this.series.clear();
    this.byteSize = 0;
    this.totalPoints = 0;
    this.startedAt = null;
  }

  evict() {
    for (const values of this.series.values()) {
      while (values.length > this.limits.maxPointsPerSeries) {
        values.shift();
        this.totalPoints -= 1;
      }
    }
    this.recountBytes();
    while (
      this.totalPoints > this.limits.maxTotalPoints ||
      this.byteSize > this.limits.maxBytes
    ) {
      let oldest = null;
      for (const [name, values] of this.series) {
        if (values.length > 0 && (oldest === null || values[0].at < oldest.at)) {
          oldest = { name, at: values[0].at };
        }
      }
      if (oldest === null) break;
      this.series.get(oldest.name).shift();
      this.totalPoints -= 1;
      this.recountBytes();
    }
  }

  recountBytes() {
    this.byteSize = new TextEncoder().encode(JSON.stringify([...this.series])).byteLength;
  }

  snapshot() {
    return Object.freeze({
      epoch: this.epoch,
      startedAt: this.startedAt,
      memberSet: [...this.lastMemberSet],
      seriesCount: this.series.size,
      totalPoints: this.totalPoints,
      byteSize: this.byteSize,
    });
  }

  points(name) {
    return [...(this.series.get(name) ?? [])];
  }
}

export function finiteNumber(value) {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

export function shouldPauseCollection(hidden, online, options = {}) {
  return !online || (hidden && options.disableHiddenPause !== true);
}

function readPath(value, path) {
  return path.split(".").reduce((current, key) => current?.[key], value);
}

function boundedMemberSet(members) {
  if (!Array.isArray(members)) return [];
  return members
    .slice(0, HISTORY_LIMITS.maxSeries)
    .map((member) => String(member?.node ?? member?.node_id ?? "unknown"))
    .sort();
}
