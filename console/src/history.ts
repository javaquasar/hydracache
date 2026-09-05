export const HISTORY_LIMITS = Object.freeze({
  maxSeries: 24,
  maxPointsPerSeries: 360,
  maxTotalPoints: 4_320,
  maxBytes: 262_144,
});

export const SERIES_DEFINITIONS: readonly (readonly [string, string, SeriesKind])[] = Object.freeze([
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

export type SeriesKind = "counter" | "gauge";
export type HistoryPoint = {
  at: number;
  value: number | null;
  raw: number | null;
  missing: boolean;
  reset: boolean;
};

export type HistoryLimits = {
  maxSeries: number;
  maxPointsPerSeries: number;
  maxTotalPoints: number;
  maxBytes: number;
};

type HistoryOptions = {
  limits?: Partial<HistoryLimits>;
  disableEviction?: boolean;
};

export class SnapshotHistory {
  readonly limits: Readonly<HistoryLimits>;
  readonly disableEviction: boolean;
  epoch: number | null = null;
  startedAt: number | null = null;
  lastMemberSet: string[] = [];
  series = new Map<string, HistoryPoint[]>();
  private readonly encodedPointBytes = new WeakMap<HistoryPoint, number>();
  private readonly encodedNameBytes = new Map<string, number>();
  byteSize = 0;
  totalPoints = 0;

  constructor(options: HistoryOptions = {}) {
    this.limits = Object.freeze({ ...HISTORY_LIMITS, ...(options.limits ?? {}) });
    this.disableEviction = options.disableEviction === true;
  }

  ingest(snapshot: Record<string, unknown>, capturedAt = Date.now()) {
    const epoch = finiteNumber(snapshot.authority_epoch ?? readPath(snapshot, "cluster.authority_epoch"));
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
      const point = { at: capturedAt, value, raw, missing: raw === null, reset };
      values.push(point);
      this.encodedPointBytes.set(point, encodedLength(JSON.stringify(point)));
      if (!this.encodedNameBytes.has(name)) {
        this.encodedNameBytes.set(name, encodedLength(JSON.stringify(name)));
      }
      this.series.set(name, values);
      this.totalPoints += 1;
    }
    if (!this.disableEviction) {
      this.evict();
    } else {
      this.recountBytes();
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
    while (this.totalPoints > this.limits.maxTotalPoints) {
      if (!this.dropOldest()) break;
    }
    this.recountBytes();
    while (this.byteSize > this.limits.maxBytes) {
      if (!this.dropOldest()) break;
      this.recountBytes();
    }
  }

  private dropOldest(): boolean {
      let oldest = null;
      for (const [name, values] of this.series) {
        if (values.length > 0 && (oldest === null || values[0]!.at < oldest.at)) {
          oldest = { name, at: values[0]!.at };
        }
      }
      if (oldest === null) return false;
      this.series.get(oldest.name)!.shift();
      this.totalPoints -= 1;
      return true;
  }

  recountBytes() {
    let bytes = 2; // Outer array brackets.
    let seriesIndex = 0;
    for (const [name, values] of this.series) {
      if (seriesIndex > 0) bytes += 1; // Entry separator.
      const nameBytes = this.encodedNameBytes.get(name) ?? encodedLength(JSON.stringify(name));
      // Entry open + encoded name + comma + points open + points close + entry close.
      bytes += 5 + nameBytes;
      for (let pointIndex = 0; pointIndex < values.length; pointIndex += 1) {
        if (pointIndex > 0) bytes += 1;
        const point = values[pointIndex]!;
        bytes += this.encodedPointBytes.get(point) ?? encodedLength(JSON.stringify(point));
      }
      seriesIndex += 1;
    }
    this.byteSize = bytes;
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

  points(name: string): HistoryPoint[] {
    return [...(this.series.get(name) ?? [])];
  }
}

function encodedLength(value: string): number {
  return new TextEncoder().encode(value).byteLength;
}

export function finiteNumber(value: unknown): number | null {
  return typeof value === "number" && Number.isFinite(value) ? value : null;
}

export function shouldPauseCollection(
  hidden: boolean,
  online: boolean,
  options: { disableHiddenPause?: boolean } = {},
): boolean {
  return !online || (hidden && options.disableHiddenPause !== true);
}

function readPath(value: unknown, path: string): unknown {
  return path.split(".").reduce<unknown>((current, key) => {
    if (typeof current !== "object" || current === null) return undefined;
    return (current as Record<string, unknown>)[key];
  }, value);
}

function boundedMemberSet(members: unknown): string[] {
  if (!Array.isArray(members)) return [];
  return members
    .slice(0, HISTORY_LIMITS.maxSeries)
    .map((member) => {
      if (typeof member !== "object" || member === null) return "unknown";
      const row = member as Record<string, unknown>;
      return String(row.node ?? row.node_id ?? "unknown");
    })
    .sort();
}
