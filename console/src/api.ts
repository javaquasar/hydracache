export const MANAGEMENT_ENDPOINTS = Object.freeze({
  dashboard: "/management/v1/dashboard",
  formation: "/management/v1/cluster/formation?limit=100",
  members: "/management/v1/cluster/members?limit=100",
  partitions: "/management/v1/cluster/partitions",
  clients: "/management/v1/clients",
  namespaces: "/management/v1/namespaces?limit=100",
  health: "/management/v1/healthchecks?limit=100",
  consensus: "/management/v1/consensus/progress?limit=100",
  recovery: "/management/v1/persistence/recovery?limit=100",
  persistence: "/management/v1/persistence",
  operations: "/management/v1/operations?limit=100",
  audit: "/management/v1/audit?limit=100",
});

export const MANAGEMENT_HEADERS = Object.freeze({
  "x-hydracache-client-id": "management-console",
  "x-hydracache-tenant": "operator",
  "x-hydracache-management-read": "true",
});

export interface ManagementEnvelope<T = unknown> {
  schema_version: number;
  observation_seq: number;
  authority_epoch: number | null;
  captured_at_unix_ms: number;
  source: unknown;
  completeness: unknown;
  stale_after_ms: number;
  warnings: readonly unknown[];
  data: T;
}

export async function fetchManagementEnvelope(
  url: string,
  signal: AbortSignal,
): Promise<ManagementEnvelope<Record<string, unknown>>> {
  if (!url.startsWith("/management/v1/")) {
    throw new Error("management console refused a non-management URL");
  }
  const response = await fetch(url, {
    cache: "no-store",
    credentials: "same-origin",
    headers: MANAGEMENT_HEADERS,
    method: "GET",
    redirect: "error",
    signal,
  });
  if (!response.ok) throw new Error(`${url} returned ${response.status}`);
  const value: unknown = await response.json();
  if (!isEnvelope(value)) throw new Error(`${url} returned an invalid management envelope`);
  if (value.schema_version !== 1) {
    throw new Error(`${url} returned unsupported schema ${value.schema_version}`);
  }
  return value;
}

export function isEnvelope(value: unknown): value is ManagementEnvelope<Record<string, unknown>> {
  if (typeof value !== "object" || value === null) return false;
  const row = value as Record<string, unknown>;
  return (
    Number.isSafeInteger(row.schema_version) &&
    Number.isSafeInteger(row.observation_seq) &&
    (row.authority_epoch === null || Number.isSafeInteger(row.authority_epoch)) &&
    Number.isSafeInteger(row.captured_at_unix_ms) &&
    Number.isSafeInteger(row.stale_after_ms) &&
    Array.isArray(row.warnings) &&
    typeof row.data === "object" &&
    row.data !== null
  );
}

export function backoffDelay(
  failures: number,
  random: () => number = Math.random,
  baseMs = 10_000,
  maximumMs = 60_000,
): number {
  const boundedFailures = Math.max(0, Math.min(Math.trunc(failures), 4));
  const exponential = Math.min(maximumMs, baseMs * 2 ** boundedFailures);
  return Math.round(exponential * (0.85 + random() * 0.3));
}
