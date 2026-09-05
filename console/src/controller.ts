import { HISTORY_LIMITS, SnapshotHistory, shouldPauseCollection } from "./history";
import {
  MANAGEMENT_ENDPOINTS,
  backoffDelay,
  fetchManagementEnvelope,
} from "./api";
import { normalizeObservationSource } from "./state";

const MAX_RENDERED_MEMBERS = 48;
const BASE_POLL_INTERVAL_MS = 10_000;
type WireValue = ReturnType<typeof JSON.parse>;
type RenderValues = Record<string, WireValue>;
type ManagedElement = HTMLElement & HTMLInputElement & SVGSVGElement;
type ControllerState = {
  timer: ReturnType<typeof setTimeout> | null;
  controller: AbortController | null;
  failures: number;
  paused: boolean;
  refreshes: number;
  health: WireValue | null;
};

declare global {
  interface Window {
    __HC_CONSOLE_STATE__: {
      state: ControllerState;
      history: SnapshotHistory;
      limits: typeof HISTORY_LIMITS;
    };
  }
}

const history = new SnapshotHistory();
const state: ControllerState = {
  timer: null,
  controller: null,
  failures: 0,
  paused: false,
  refreshes: 0,
  health: null,
};
window.__HC_CONSOLE_STATE__ = { state, history, limits: HISTORY_LIMITS };

export function startController() {
  for (const region of document.querySelectorAll(".table-scroll")) {
    (region as HTMLElement).tabIndex = 0;
  }
  wireLifecycle();
  wireHealthFilters();
  void refresh();
}

function wireHealthFilters() {
  for (const id of ["health-search", "health-status-filter", "health-category-filter"]) {
    byTest(id).addEventListener("input", () => renderHealth(state.health));
  }
}

function wireLifecycle() {
  const reevaluate = () => {
    const paused = shouldPauseCollection(document.hidden, navigator.onLine);
    state.paused = paused;
    if (paused) {
      if (state.timer !== null) clearTimeout(state.timer);
      state.controller?.abort();
      setText("poll-state", document.hidden ? "paused while hidden" : "paused while offline");
    } else {
      schedule(0);
    }
  };
  document.addEventListener("visibilitychange", reevaluate);
  window.addEventListener("online", reevaluate);
  window.addEventListener("offline", reevaluate);
}

async function refresh() {
  if (state.paused || document.hidden || !navigator.onLine) return;
  state.controller?.abort();
  const controller = new AbortController();
  state.controller = controller;
  setText("poll-state", "refreshing typed snapshots");
  try {
    const historyEnd = Date.now();
    const requestEndpoints = {
      ...MANAGEMENT_ENDPOINTS,
      remoteHistory: `/management/v1/history?query_id=cache_entries&start_ms=${historyEnd - 3_600_000}&end_ms=${historyEnd}&step_ms=60000`,
    };
    const results = await Promise.allSettled(
      Object.entries(requestEndpoints).map(async ([name, url]) => [
        name,
        await fetchManagementEnvelope(url, controller.signal),
      ] as const),
    );
    const values: RenderValues = Object.fromEntries(
      results.filter((item) => item.status === "fulfilled").map((item) => item.value),
    );
    if (!values.dashboard) throw new Error("dashboard snapshot unavailable");
    const traceId = values.partitions?.data?.placement_trace_id;
    if (typeof traceId === "string" && /^[A-Za-z0-9_-]{1,128}$/.test(traceId)) {
      try {
        values.placementTrace = await fetchManagementEnvelope(
          `/management/v1/cluster/placement-traces/${encodeURIComponent(traceId)}`,
          controller.signal,
        );
      } catch (error) {
        if (isAbortError(error)) throw error;
      }
    }
    const namespace = values.namespaces?.data?.items?.[0]?.namespace;
    if (
      typeof namespace === "string" &&
      namespace.length > 0 &&
      namespace.length <= 128 &&
      !containsControlCharacter(namespace)
    ) {
      try {
        values.namespaceCaches = await fetchManagementEnvelope(
          `/management/v1/namespaces/${encodeURIComponent(namespace)}/caches`,
          controller.signal,
        );
      } catch (error) {
        if (isAbortError(error)) throw error;
      }
    }
    render(values);
    state.failures = 0;
    state.refreshes += 1;
    schedule(BASE_POLL_INTERVAL_MS);
  } catch (error) {
    if (isAbortError(error)) return;
    state.failures += 1;
    renderDegraded(error);
    schedule(backoffDelay(state.failures));
  }
}

function schedule(delay: number) {
  if (state.timer !== null) clearTimeout(state.timer);
  if (state.paused || document.hidden || !navigator.onLine) return;
  state.timer = setTimeout(refresh, delay);
}

function render(values: RenderValues) {
  const envelope = values.dashboard;
  const data = envelope.data;
  const source = normalizeObservationSource(envelope.source);
  badge(source);
  setText("poll-state", `last refresh ${new Date().toLocaleTimeString()}`);
  const degraded = byTest("degraded-state");
  degraded.hidden = true;
  renderWarnings(Object.values(values));
  renderSummary(data, values);
  renderMetrics(data);
  renderMembers(values.members?.data?.items ?? data.members ?? []);
  renderFormation(values.formation);
  renderPartitions(values.partitions, data.partitions);
  renderClients(values.clients);
  renderNamespaces(values.namespaces, values.namespaceCaches);
  state.health = values.health;
  renderHealth(values.health);
  renderConsensus(values.consensus, data.consensus);
  renderRecovery(values.recovery);
  renderPersistence(values.persistence);
  renderOperations(values.operations);
  renderAudit(values.audit);
  renderPlacement(values.placementTrace?.data, data.placement);
  renderRemoteHistory(values.remoteHistory);
  history.ingest(
    { ...data, authority_epoch: envelope.authority_epoch },
    envelope.captured_at_unix_ms,
  );
  renderHistory();
}

function renderRemoteHistory(envelope: WireValue) {
  const data = envelope?.data;
  const state = data?.state ?? "no_adapter";
  const source = data?.source ?? "browser_local";
  const series = Array.isArray(data?.series) ? data.series.length : 0;
  setText(
    "remote-history-state",
    state === "available" || state === "partial"
      ? `${source} · ${state} · ${series} bounded series (kept separate)`
      : `${state} · using browser-local history`,
  );
}

function renderHealth(envelope: WireValue) {
  const data = envelope?.data ?? {};
  const checks = data.checks?.items ?? [];
  const search = byTest("health-search").value.trim().toLocaleLowerCase();
  const statusFilter = byTest("health-status-filter").value;
  const categoryFilter = byTest("health-category-filter").value;
  const visible = checks
    .filter((check: WireValue) => !statusFilter || check.status === statusFilter)
    .filter((check: WireValue) => !categoryFilter || check.category === categoryFilter)
    .filter(
      (check: WireValue) =>
        !search ||
        `${check.id} ${check.title} ${check.remediation_code}`.toLocaleLowerCase().includes(search),
    )
    .slice(0, MAX_RENDERED_MEMBERS);
  const counts = data.counts ?? {};
  byTest("health-counts").replaceChildren(
    ...["fail", "warn", "unknown", "pass", "disabled"].map((name) =>
      pill(name.toUpperCase(), known(counts[name])),
    ),
  );
  const aggregate = byTest("health-aggregate");
  aggregate.textContent = data.aggregate ?? "UNKNOWN";
  aggregate.className = `truth-chip ${(data.aggregate ?? "UNKNOWN").toLocaleLowerCase()}`;
  byTest("health-table").replaceChildren(
    ...visible.map((check: WireValue) =>
      row(
        [
          check.id,
          status(check.status),
          check.category,
          check.title,
          (check.evidence ?? [])
            .map((item: WireValue) =>
              item.value == null ? item.code : `${item.code}=${item.value}${item.unit ? ` ${item.unit}` : ""}`,
            )
            .join(", ") || "none",
          check.affected_count == null ? "none" : known(check.affected_count),
          check.remediation_code,
          known(check.observation_seq),
        ],
        { testid: "health-row" },
      ),
    ),
  );
  if (visible.length === 0) {
    byTest("health-table").append(
      row(["No matching checks", "UNKNOWN", "unknown", "No server verdict", "source-unavailable", "none", "inspect-source", "unavailable"]),
    );
  }
}

function renderWarnings(envelopes: WireValue[]) {
  const warnings = envelopes.flatMap((value) => value?.warnings ?? []);
  const strip = byTest("truth-warnings");
  strip.hidden = warnings.length === 0;
  strip.replaceChildren(...warnings.map((warning) => chip(warning.code ?? "unknown", "warning")));
}

function renderSummary(data: WireValue, values: RenderValues) {
  metric(
    "cluster-state",
    "Cluster state",
    data.cluster?.state ?? "unavailable",
    `quorum ${truth(data.cluster?.quorum_ok)} · authority ${truth(data.cluster?.metadata_authoritative)}`,
  );
  metric(
    "leader",
    "Leader",
    data.cluster?.leader ?? "electing",
    `term ${known(data.cluster?.term)} / epoch ${known(data.cluster?.authority_epoch)}`,
  );
  metric(
    "partition-summary",
    "Replication",
    format(data.replication?.success_total),
    `failed ${format(data.replication?.failure_total)} · under-replicated ${format(data.replication?.under_replicated)}`,
  );
  metric(
    "member-summary",
    "Members",
    format(data.cluster?.member_count),
    `${format(data.cluster?.voter_count)} voters · snapshot ${values.dashboard.completeness}`,
  );
  const formation = values.formation?.data;
  const formationItems = formation?.items ?? [];
  const blocked = formationItems.filter((item: WireValue) => item.serving === "blocked").length;
  const discovered = formationItems.filter((item: WireValue) => item.discovery !== "absent").length;
  const authenticated = formationItems.filter((item: WireValue) => item.transport === "authenticated").length;
  const admitted = formationItems.filter((item: WireValue) => item.admission === "admitted").length;
  const current = formationItems.filter((item: WireValue) => item.catch_up === "current").length;
  const serving = formationItems.filter((item: WireValue) => item.serving === "serving").length;
  const unknown = formationItems.filter((item: WireValue) =>
    [item.transport, item.admission, item.consensus_role, item.catch_up, item.serving].includes(
      "unknown",
    ),
  ).length;
  metric(
    "formation-summary",
    "Formation",
    `${serving} serving`,
    `${discovered} discovered · ${authenticated} auth · ${admitted} admitted · ${current} current · ${blocked} blocked · ${unknown} unknown${formation?.truncated ? " · truncated" : ""}`,
  );
  const lag = data.consensus?.apply_lag;
  metric(
    "consensus-summary",
    "Raft apply lag",
    known(lag),
    lag == null
      ? "source unavailable"
      : `${known(data.consensus.commit_index)} committed / ${known(data.consensus.applied_index)} applied`,
  );
  const recoveries = values.recovery?.data?.items ?? [];
  metric(
    "recovery-summary",
    "Recovery",
    `${recoveries.length} observed`,
    recoveryLabel(recoveries),
  );
  const placement = data.placement;
  metric(
    "placement-summary",
    "Placement",
    placement?.outcome ?? "unavailable",
    placement?.selected == null
      ? "source unavailable"
      : `${placement.selected} selected · ${placement.rejected ?? "unknown"} rejected`,
  );
}

function renderMetrics(data: WireValue) {
  const values: Array<[string, string]> = [
    [
      "hit ratio",
      data.cache?.hit_ratio == null ? "unavailable" : `${(data.cache.hit_ratio * 100).toFixed(1)}%`,
    ],
    ["entries", known(data.cache?.entries)],
    ["retained bytes", bytes(data.cache?.retained_bytes)],
    ["loads", known(data.cache?.loads_total)],
    ["admission rejects", known(data.cache?.admission_rejected_total)],
    ["queue depth", known(data.cache?.admission_queue_depth)],
    ["repair debt", known(data.replication?.repair_debt)],
    ["zone underspread", known(data.replication?.zone_underspread)],
    ["partitions", known(data.partitions?.total)],
    ["unassigned", known(data.partitions?.unassigned)],
  ];
  byTest("metrics-strip").replaceChildren(...values.map(([label, value]) => pill(label, value)));
  const facts = byTest("lifecycle-panel");
  facts.replaceChildren(
    fact("Reshard phase", data.reshard?.phase),
    fact("Moves in flight", data.reshard?.moves_inflight),
    fact("Backfill lag", data.reshard?.backfill_lag),
    fact("TTL backlog", data.cache?.ttl_backlog),
  );
}

function renderMembers(members: WireValue[]) {
  const rendered = members.slice(0, MAX_RENDERED_MEMBERS);
  setText(
    "render-cap",
    rendered.length < members.length
      ? `${rendered.length} rendered, ${members.length - rendered.length} not rendered`
      : `${rendered.length} rendered`,
  );
  byTest("members-list").replaceChildren(
    ...rendered.map((member) =>
      row(
        [
          member.node,
          member.consensus_role ?? member.role ?? "unknown",
          status(member.reachability),
          known(member.generation),
          known(member.cpu_percent, "%"),
          bytes(member.rss_bytes),
          bytes(member.retained_bytes),
          duration(member.uptime_seconds),
          known(member.client_count),
          known(member.partition_count),
          member.config_digest ?? "unavailable",
        ],
        { testid: "member", reachability: member.reachability },
      ),
    ),
  );
}

function renderPartitions(envelope: WireValue, fallback: WireValue) {
  const snapshot = envelope?.data ?? fallback ?? {};
  byTest("partition-details").replaceChildren(
    pill("total", known(snapshot.total)),
    pill("assigned", known(snapshot.assigned)),
    pill("unassigned", known(snapshot.unassigned)),
    pill("under replicated", known(snapshot.under_replicated)),
    pill("zone underspread", known(snapshot.zone_underspread)),
    pill("repair debt", known(snapshot.repair_debt)),
    pill("moves", known(snapshot.reshard_moves_inflight)),
    pill("backfill lag", known(snapshot.backfill_lag)),
  );
  const distribution = Array.isArray(snapshot.distribution) ? snapshot.distribution : [];
  byTest("partition-table").replaceChildren(
    ...distribution.slice(0, MAX_RENDERED_MEMBERS).map((item: WireValue) =>
      row([item.node, known(item.primary), known(item.backup)], {
        testid: "partition-row",
      }),
    ),
  );
  if (distribution.length === 0) {
    byTest("partition-table").append(
      row(["Ownership source unavailable", "unavailable", "unavailable"]),
    );
  }
}

function renderClients(envelope: WireValue) {
  const clients = envelope?.data ?? {};
  byTest("client-details").replaceChildren(
    pill("active", known(clients.active_connections)),
    pill("accepted", known(clients.accepted_total)),
    pill("closed", known(clients.closed_total)),
    pill("rejected", known(clients.rejected_total)),
    pill("pending", known(clients.pending_invocations)),
    pill("subscriptions", known(clients.active_subscriptions)),
    pill("sessions", known(clients.active_sessions)),
    pill("buffered bytes", bytes(clients.buffered_bytes)),
  );
  const protocols = Array.isArray(clients.protocols) ? clients.protocols : [];
  byTest("client-table").replaceChildren(
    ...protocols.map((protocol: WireValue) =>
      row(
        [
          protocol.protocol,
          protocol.version ?? "unavailable",
          known(protocol.active_connections),
          known(protocol.accepted_total),
          known(protocol.closed_total),
          known(protocol.rejected_total),
          known(protocol.pending_invocations),
        ],
        { testid: "client-protocol-row" },
      ),
    ),
  );
  if (protocols.length === 0) {
    byTest("client-table").append(
      row(["No protocol source", "unavailable", "unavailable", "unavailable", "unavailable", "unavailable", "unavailable"]),
    );
  }
}

function renderNamespaces(envelope: WireValue, cacheEnvelope: WireValue) {
  const namespaces = envelope?.data?.items ?? [];
  byTest("namespace-table").replaceChildren(
    ...namespaces.slice(0, MAX_RENDERED_MEMBERS).map((namespace: WireValue) =>
      row(
        [
          namespace.namespace,
          known(namespace.cache_count),
          known(namespace.entries),
          bytes(namespace.logical_bytes),
          bytes(namespace.retained_bytes),
          `${known(namespace.entries)} / ${known(namespace.max_entries)}`,
          `${bytes(namespace.logical_bytes)} / ${bytes(namespace.max_bytes)}`,
          known(namespace.admission_rejected_total),
          namespace.persistence_status ?? "unavailable",
        ],
        { testid: "namespace-row" },
      ),
    ),
  );
  if (namespaces.length === 0) {
    byTest("namespace-table").append(
      row(["No authorized namespace source", "unavailable", "unavailable", "unavailable", "unavailable", "unavailable", "unavailable", "unavailable", "unavailable"]),
    );
  }
  const caches = cacheEnvelope?.data?.items ?? [];
  byTest("cache-table").replaceChildren(
    ...caches.map((cache: WireValue) =>
      row(
        [
          cache.namespace,
          cache.cache,
          known(cache.entries),
          bytes(cache.logical_bytes),
          bytes(cache.retained_bytes),
          known(cache.ttl_backlog),
          known(cache.idempotency_records),
          known(cache.backup_age_seconds),
        ],
        { testid: "cache-row" },
      ),
    ),
  );
  if (caches.length === 0) {
    byTest("cache-table").append(
      row(["No cache detail", "unavailable", "unavailable", "unavailable", "unavailable", "unavailable", "unavailable", "unavailable"]),
    );
  }
}

function renderFormation(envelope: WireValue) {
  const items = envelope?.data?.items ?? [];
  byTest("formation-table").replaceChildren(
    ...items.map((item: WireValue) =>
      row(
        [
          item.node,
          status(item.transport),
          status(item.admission),
          `${item.consensus_role} / ${item.catch_up}`,
          status(item.serving),
          item.blocker ?? "none",
        ],
        { testid: "formation-row" },
      ),
    ),
  );
  if (items.length === 0) {
    byTest("formation-table").append(
      row([
        "No formation evidence",
        "unavailable",
        "unavailable",
        "unavailable",
        "blocked",
        "source-unavailable",
      ]),
    );
  }
}

function renderConsensus(envelope: WireValue, local: WireValue) {
  const items = envelope?.data?.items ?? [];
  byTest("consensus-table").replaceChildren(
    ...items.map((item: WireValue) => {
      const lag =
        item.commit_index == null || item.applied_index == null
          ? null
          : Math.max(0, item.commit_index - item.applied_index);
      return row(
        [
          item.node,
          known(item.commit_index),
          known(item.applied_index),
          known(lag),
          status(lag === 0 ? "current" : lag == null ? "unavailable" : "behind"),
        ],
        { testid: "consensus-row" },
      );
    }),
  );
  if (items.length === 0) {
    byTest("consensus-table").append(
      row([
        "Local summary",
        known(local?.commit_index),
        known(local?.applied_index),
        known(local?.apply_lag),
        "unavailable",
      ]),
    );
  }
}

function renderRecovery(envelope: WireValue) {
  const items = envelope?.data?.items ?? [];
  const outcomes = ["clean", "repaired", "partial", "corrupt", "failed"];
  byTest("recovery-outcomes").replaceChildren(
    ...outcomes.map((outcome) =>
      pill(
        outcome,
        items.filter((item: WireValue) => item.outcome === outcome).length,
      ),
    ),
  );
  byTest("recovery-table").replaceChildren(
    ...items.map((item: WireValue) =>
      row(
        [
          item.scope,
          status(item.outcome),
          item.phase,
          boundedCount(item.corrupt_records),
          item.reason ?? "none",
        ],
        { testid: "recovery-row" },
      ),
    ),
  );
}

function renderPersistence(envelope: WireValue) {
  const data = envelope?.data ?? {};
  byTest("persistence-details").replaceChildren(
    pill("configured", truth(data.configured)),
    pill("enabled", truth(data.enabled)),
    pill("storage", data.storage_open === true ? "open" : data.storage_open === false ? "closed" : "unavailable"),
    pill("backup age", known(data.backup_age_seconds, " s")),
    pill("verified backup", data.last_verified_backup_id ?? "unavailable"),
    pill("verified restore", data.last_verified_restore_id ?? "unavailable"),
    pill("verification", data.verification_state ?? "unavailable"),
    pill("recovery", data.recovery_state ?? "unknown"),
  );
}

function renderOperations(envelope: WireValue) {
  const data = envelope?.data ?? {};
  const items = data.items?.items ?? [];
  setText(
    "operations-generation",
    `generation ${known(data.generation)} · sequence ${known(data.latest_sequence)} · evicted ${known(data.evicted_records)}`,
  );
  byTest("operations-table").replaceChildren(
    ...items.slice(0, MAX_RENDERED_MEMBERS).map((item: WireValue) =>
      row(
        [
          item.operation_id,
          item.kind,
          item.scope,
          status(item.state),
          time(item.requested_at_unix_ms),
          time(item.started_at_unix_ms),
          time(item.terminal_at_unix_ms),
          item.reason_code ?? "none",
        ],
        { testid: "operation-row" },
      ),
    ),
  );
  if (items.length === 0) {
    byTest("operations-table").append(
      row(["No retained operations", "none", "current process", "unknown", "unavailable", "unavailable", "unavailable", "none"]),
    );
  }
}

function renderAudit(envelope: WireValue) {
  const data = envelope?.data ?? {};
  const items = data.items?.items ?? [];
  setText(
    "audit-coverage",
    `${data.coverage ?? "management operations current process only"} · evicted ${known(data.evicted_records)}`,
  );
  byTest("audit-table").replaceChildren(
    ...items.slice(0, MAX_RENDERED_MEMBERS).map((item: WireValue) =>
      row(
        [
          item.event_id,
          item.operation_id,
          item.action,
          status(item.outcome),
          time(item.occurred_at_unix_ms),
          item.source,
        ],
        { testid: "audit-row" },
      ),
    ),
  );
  if (items.length === 0) {
    byTest("audit-table").append(
      row(["No retained audit metadata", "none", "none", "unknown", "unavailable", "runtime_journal"]),
    );
  }
}

function renderPlacement(trace: WireValue, fallback: WireValue) {
  const placement = trace ?? fallback;
  const state = byTest("placement-state");
  state.textContent = placement?.outcome ?? "unavailable";
  state.className = `truth-chip ${placement?.outcome ?? "unavailable"}`;
  byTest("placement-details").replaceChildren(
    pill("selected", known(Array.isArray(placement?.selected) ? placement.selected.length : placement?.selected)),
    pill("rejected", known(trace ? trace.candidates?.items?.filter((item: WireValue) => !item.selected).length : placement?.rejected)),
    pill("committed", known(trace?.commit_index ?? placement?.latest_committed_epoch)),
    pill("applied", known(trace?.applied_index ?? placement?.latest_applied_epoch)),
  );
  const candidates = trace?.candidates?.items ?? [];
  byTest("placement-table").replaceChildren(
    ...candidates.slice(0, MAX_RENDERED_MEMBERS).map((candidate: WireValue) =>
      row(
        [
          candidate.node,
          status(candidate.selected ? "selected" : "rejected"),
          candidate.reasons?.join(", ") || "none",
        ],
        { testid: "placement-row" },
      ),
    ),
  );
  if (candidates.length === 0) {
    byTest("placement-table").append(
      row(["No retained placement trace", "unavailable", "source-unavailable"]),
    );
  }
}

function renderHistory() {
  const snapshot = history.snapshot();
  setText(
    "history-budget",
    `${snapshot.totalPoints}/${HISTORY_LIMITS.maxTotalPoints} points · ${snapshot.byteSize}/${HISTORY_LIMITS.maxBytes} bytes · ${snapshot.seriesCount}/${HISTORY_LIMITS.maxSeries} series`,
  );
  for (const svg of document.querySelectorAll<SVGSVGElement>("svg[data-series]")) {
    drawSparkline(svg, history.points(svg.dataset.series ?? ""));
  }
}

function drawSparkline(svg: SVGSVGElement, points: ReturnType<SnapshotHistory["points"]>) {
  svg.replaceChildren();
  const finite = points.flatMap((point, index) =>
    point.value === null ? [] : [{ x: index, y: point.value }],
  );
  if (finite.length < 2) {
    const text = document.createElementNS("http://www.w3.org/2000/svg", "text");
    text.setAttribute("x", "16");
    text.setAttribute("y", "66");
    text.textContent = "waiting for comparable samples";
    svg.append(text);
    return;
  }
  const max = Math.max(...finite.map((point) => point.y), 1);
  const span = Math.max(points.length - 1, 1);
  const polyline = document.createElementNS("http://www.w3.org/2000/svg", "polyline");
  polyline.setAttribute(
    "points",
    finite
      .map((point) => `${10 + (point.x / span) * 460},${110 - (point.y / max) * 96}`)
      .join(" "),
  );
  svg.append(polyline);
}

function renderDegraded(error: unknown) {
  badge("unavailable");
  setText("poll-state", "cannot reach management snapshots");
  const node = byTest("degraded-state");
  node.hidden = false;
  node.textContent = `Cannot reach cluster: ${errorMessage(error)}`;
  for (const id of [
    "cluster-state",
    "leader",
    "partition-summary",
    "member-summary",
    "formation-summary",
    "consensus-summary",
    "recovery-summary",
  ]) {
    metric(id, id.replaceAll("-", " "), "unavailable", "no trusted snapshot");
  }
}

function badge(source: string) {
  const node = byTest("source-badge");
  node.textContent = source;
  node.dataset.source = source;
}
function metric(id: string, label: string, value: string, detail: string) {
  const node = byTest(id);
  node.replaceChildren();
  const a = document.createElement("span");
  a.textContent = label;
  const b = document.createElement("strong");
  b.textContent = String(value);
  const c = document.createElement("small");
  c.textContent = detail;
  node.append(a, b, c);
}
function pill(label: string, value: string | number) {
  const node = document.createElement("span");
  node.className = "metric-pill";
  const a = document.createElement("small");
  a.textContent = label;
  const b = document.createElement("strong");
  b.textContent = String(value);
  node.append(a, b);
  return node;
}
function fact(label: string, value: WireValue) {
  const wrap = document.createElement("div");
  const dt = document.createElement("dt");
  dt.textContent = label;
  const dd = document.createElement("dd");
  dd.textContent = known(value);
  wrap.append(dt, dd);
  return wrap;
}
function row(
  values: Array<string | number | Node | null | undefined>,
  data: { testid?: string; reachability?: string } = {},
) {
  const tr = document.createElement("tr");
  if (data.testid) tr.dataset.testid = data.testid;
  if (data.reachability) tr.dataset.reachability = data.reachability;
  tr.append(
    ...values.map((value) => {
      const td = document.createElement("td");
      if (value instanceof Node) td.append(value);
      else td.textContent = value == null ? "unavailable" : String(value);
      return td;
    }),
  );
  return tr;
}
function status(value: WireValue) {
  return chip(value ?? "unknown", value ?? "unknown");
}
function chip(text: string, kind: string) {
  const node = document.createElement("span");
  node.className = `truth-chip ${kind}`;
  node.textContent = text;
  return node;
}
function byTest(id: string): ManagedElement {
  const node = document.querySelector(`[data-testid='${id}']`);
  if (node === null) throw new Error(`Management Center element is missing: ${id}`);
  return node as ManagedElement;
}
function setText(id: string, text: string) {
  byTest(id).textContent = text;
}
function known(value: WireValue, suffix = "") {
  return value == null ? "unavailable" : `${new Intl.NumberFormat("en-US").format(value)}${suffix}`;
}
function format(value: WireValue) {
  return known(value);
}
function truth(value: WireValue) {
  return value === true ? "yes" : value === false ? "no" : "unknown";
}
function bytes(value: WireValue) {
  if (value == null) return "unavailable";
  if (value < 1024) return `${value} B`;
  return `${(value / 1024).toFixed(1)} KiB`;
}
function duration(value: WireValue) {
  if (value == null) return "unavailable";
  return value < 60 ? `${value}s` : `${Math.floor(value / 60)}m`;
}
function boundedCount(value: WireValue) {
  return value?.value == null ? "unavailable" : `${value.value}${value.exact ? "" : "+"}`;
}

function time(value: WireValue) {
  if (!Number.isFinite(value)) return "unavailable";
  return new Date(value).toISOString();
}
function recoveryLabel(items: WireValue[]) {
  if (items.length === 0) return "source unavailable";
  const bad = items.filter((item) => ["corrupt", "failed", "partial"].includes(item.outcome)).length;
  return bad ? `${bad} need attention` : "no adverse outcome observed";
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : "unknown management error";
}

function containsControlCharacter(value: string): boolean {
  return [...value].some((character) => {
    const code = character.charCodeAt(0);
    return code <= 0x1f || code === 0x7f;
  });
}
