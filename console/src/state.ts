export type ObservationSource = "live" | "modeled" | "unavailable";

export type ObservationIdentity = {
  authorityEpoch: number | null;
  observationSeq: number;
};

export function normalizeObservationSource(value: unknown): ObservationSource {
  return value === "live" || value === "modeled" || value === "unavailable"
    ? value
    : "unavailable";
}

export function shouldAcceptObservation(
  current: ObservationIdentity | null,
  candidate: ObservationIdentity,
): boolean {
  if (current === null) return true;
  if (current.authorityEpoch === null) return candidate.authorityEpoch !== null;
  if (candidate.authorityEpoch === null) return false;
  if (candidate.authorityEpoch !== current.authorityEpoch) {
    return candidate.authorityEpoch > current.authorityEpoch;
  }
  return candidate.observationSeq > current.observationSeq;
}
