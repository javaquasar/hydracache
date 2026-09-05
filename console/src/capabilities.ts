export type CapabilityId = "cluster_formation" | "consensus_progress" | "persistence_recovery";

export interface CapabilityView {
  id: CapabilityId;
  route: "formation" | "consensus" | "recovery";
  endpoint: "formation" | "consensus" | "recovery";
  available: boolean;
  reason: string;
}

const CAPABILITY_VIEWS = Object.freeze([
  { id: "cluster_formation", route: "formation", endpoint: "formation" },
  { id: "consensus_progress", route: "consensus", endpoint: "consensus" },
  { id: "persistence_recovery", route: "recovery", endpoint: "recovery" },
] as const);

export function capabilityViews(data: unknown): readonly CapabilityView[] {
  const entries = isRecord(data) && Array.isArray(data.capabilities) ? data.capabilities : [];
  return CAPABILITY_VIEWS.map((definition) => {
    const row = entries.find((candidate) => isRecord(candidate) && candidate.id === definition.id);
    const availability = isRecord(row) && typeof row.availability === "string" ? row.availability : "unavailable";
    const reason = isRecord(row) && typeof row.reason === "string" ? row.reason : "capability-not-advertised";
    return {
      ...definition,
      available: availability === "available" || availability === "partial",
      reason,
    };
  });
}

export function capabilityAllowsEndpoint(endpoint: string, data: unknown): boolean {
  const view = capabilityViews(data).find((candidate) => candidate.endpoint === endpoint);
  return view?.available ?? true;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}
