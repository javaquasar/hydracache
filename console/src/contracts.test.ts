import { describe, expect, it, vi } from "vitest";
import { backoffDelay, fetchManagementEnvelope, isEnvelope, MANAGEMENT_HEADERS } from "./api";
import { routeFromHash } from "./router";
import { normalizeObservationSource, shouldAcceptObservation } from "./state";
import { capabilityAllowsEndpoint, capabilityViews } from "./capabilities";

const envelope = {
  schema_version: 1,
  observation_seq: 2,
  authority_epoch: 7,
  captured_at_unix_ms: 10,
  source: "live",
  completeness: "complete",
  stale_after_ms: 2_000,
  warnings: [],
  data: {},
};

describe("typed management contracts", () => {
  it("rejects malformed and unsupported envelopes", async () => {
    expect(isEnvelope(envelope)).toBe(true);
    expect(isEnvelope({ ...envelope, warnings: "none" })).toBe(false);
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: true, json: async () => ({ ...envelope, schema_version: 2 }) }));
    await expect(fetchManagementEnvelope("/management/v1/dashboard", new AbortController().signal)).rejects.toThrow("unsupported schema 2");
    expect(fetch).toHaveBeenCalledWith("/management/v1/dashboard", expect.objectContaining({ method: "GET", redirect: "error", headers: MANAGEMENT_HEADERS }));
    vi.unstubAllGlobals();
  });

  it("refuses URLs outside the fixed management prefix before fetch", async () => {
    const fetchSpy = vi.fn();
    vi.stubGlobal("fetch", fetchSpy);
    await expect(fetchManagementEnvelope("http://169.254.169.254/", new AbortController().signal)).rejects.toThrow("refused");
    expect(fetchSpy).not.toHaveBeenCalled();
    vi.unstubAllGlobals();
  });

  it("bounds deterministic backoff jitter", () => {
    expect(backoffDelay(0, () => 0)).toBe(8_500);
    expect(backoffDelay(100, () => 1)).toBe(69_000);
  });

  it("routes only to declared sections", () => {
    expect(routeFromHash("#members")).toBe("members");
    expect(routeFromHash("#unknown")).toBe("dashboard");
  });

  it("maps absent and unavailable capabilities to hidden views without issuing their requests", () => {
    const views = capabilityViews({
      capabilities: [
        { id: "cluster_formation", availability: "partial", reason: "partial-observation" },
        { id: "persistence_recovery", availability: "unavailable", reason: "status-not-retained" },
      ],
    });
    expect(views.find((view) => view.id === "cluster_formation")).toMatchObject({ available: true });
    expect(views.find((view) => view.id === "consensus_progress")).toMatchObject({ available: false, reason: "capability-not-advertised" });
    expect(capabilityAllowsEndpoint("recovery", { capabilities: [{ id: "persistence_recovery", availability: "unavailable" }] })).toBe(false);
    expect(capabilityAllowsEndpoint("dashboard", {})).toBe(true);
  });

  it("rejects source and observation regression", () => {
    expect(normalizeObservationSource("future-live")).toBe("unavailable");
    expect(shouldAcceptObservation({ authorityEpoch: 7, observationSeq: 2 }, { authorityEpoch: 7, observationSeq: 1 })).toBe(false);
    expect(shouldAcceptObservation({ authorityEpoch: 7, observationSeq: 2 }, { authorityEpoch: 8, observationSeq: 1 })).toBe(true);
  });
});
