import { HISTORY_LIMITS, SnapshotHistory, shouldPauseCollection } from "./history.js";

const MAX_RENDERED_MEMBERS = 48;
const BASE_POLL_INTERVAL_MS = 10_000;
const MAX_BACKOFF_MS = 60_000;
const ENDPOINTS = Object.freeze({
  dashboard: "/management/v1/dashboard",
  formation: "/management/v1/cluster/formation?limit=100",
  members: "/management/v1/cluster/members?limit=100",
  partitions: "/management/v1/cluster/partitions",
  clients: "/management/v1/clients",
  namespaces: "/management/v1/namespaces?limit=100",
  health: "/management/v1/healthchecks?limit=100",
  consensus: "/management/v1/consensus/progress?limit=100",
  recovery: "/management/v1/persistence/recovery?limit=100",
});
const ADMIN_HEADERS = Object.freeze({
  "x-hydracache-client-id": "management-console",
  "x-hydracache-tenant": "operator",
  "x-hydracache-admin": "true",
});

const history = new SnapshotHistory();
const state = { timer: null, controller: null, failures: 0, paused: false, refreshes: 0, health: null };
window.__HC_CONSOLE_STATE__ = { state, history, limits: HISTORY_LIMITS };

document.addEventListener("DOMContentLoaded", () => {
  wireLifecycle();
  wireHealthFilters();
  refresh();
});

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
      clearTimeout(state.timer);
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
  state.controller = new AbortController();
  setText("poll-state", "refreshing typed snapshots");
  try {
    const results = await Promise.allSettled(
      Object.entries(ENDPOINTS).map(async ([name, url]) => [
        name,
        await fetchEnvelope(url, state.controller.signal),
      ]),
    );
    const values = Object.fromEntries(
      results.filter((item) => item.status === "fulfilled").map((item) => item.value),
    );
    if (!values.dashboard) throw new Error("dashboard snapshot unavailable");
    const traceId = values.partitions?.data?.placement_trace_id;
    if (typeof traceId === "string" && /^[A-Za-z0-9_-]{1,128}$/.test(traceId)) {
      try {
        values.placementTrace = await fetchEnvelope(
          `/management/v1/cluster/placement-traces/${encodeURIComponent(traceId)}`,
          state.controller.signal,
        );
      } catch (error) {
        if (error.name === "AbortError") throw error;
      }
    }
    const namespace = values.namespaces?.data?.items?.[0]?.namespace;
    if (
      typeof namespace === "string" &&
      namespace.length > 0 &&
      namespace.length <= 128 &&
      !/[\u0000-\u001f\u007f]/.test(namespace)
    ) {
      try {
        values.namespaceCaches = await fetchEnvelope(
          `/management/v1/namespaces/${encodeURIComponent(namespace)}/caches`,
          state.controller.signal,
        );
      } catch (error) {
        if (error.name === "AbortError") throw error;
      }
    }
    render(values);
    state.failures = 0;
    state.refreshes += 1;
    schedule(BASE_POLL_INTERVAL_MS);
  } catch (error) {
    if (error.name === "AbortError") return;
    state.failures += 1;
    renderDegraded(error);
    schedule(backoffDelay(state.failures));
  }
}

async function fetchEnvelope(url, signal) {
  const response = await fetch(url, { cache: "no-store", headers: ADMIN_HEADERS, signal });
  if (!response.ok) throw new Error(`${url} returned ${response.status}`);
  return response.json();
}

function schedule(delay) {
  clearTimeout(state.timer);
  if (state.paused || document.hidden || !navigator.onLine) return;
  state.timer = setTimeout(refresh, delay);
}

function backoffDelay(failures) {
  const exponential = Math.min(
    MAX_BACKOFF_MS,
    BASE_POLL_INTERVAL_MS * 2 ** Math.min(failures, 4),
  );
  return Math.round(exponential * (0.85 + Math.random() * 0.3));
}

function render(values) {
  const envelope = values.dashboard;
  const data = envelope.data;
  const source = ["live", "modeled", "unavailable"].includes(envelope.source)
    ? envelope.source
    : "unavailable";
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
  renderPlacement(values.placementTrace?.data, data.placement);
  history.ingest(
    { ...data, authority_epoch: envelope.authority_epoch },
    envelope.captured_at_unix_ms,
  );
  renderHistory();
}

function renderHealth(envelope) {
  const data = envelope?.data ?? {};
  const checks = data.checks?.items ?? [];
  const search = byTest("health-search").value.trim().toLocaleLowerCase();
  const statusFilter = byTest("health-status-filter").value;
  const categoryFilter = byTest("health-category-filter").value;
  const visible = checks
    .filter((check) => !statusFilter || check.status === statusFilter)
    .filter((check) => !categoryFilter || check.category === categoryFilter)
    .filter(
      (check) =>
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
    ...visible.map((check) =>
      row(
        [
          check.id,
          status(check.status),
          check.category,
          check.title,
          (check.evidence ?? [])
            .map((item) =>
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

function renderWarnings(envelopes) {
  const warnings = envelopes.flatMap((value) => value?.warnings ?? []);
  const strip = byTest("truth-warnings");
  strip.hidden = warnings.length === 0;
  strip.replaceChildren(...warnings.map((warning) => chip(warning.code ?? "unknown", "warning")));
}

function renderSummary(data, values) {
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
  const blocked = formationItems.filter((item) => item.serving === "blocked").length;
  const discovered = formationItems.filter((item) => item.discovery !== "absent").length;
  const authenticated = formationItems.filter((item) => item.transport === "authenticated").length;
  const admitted = formationItems.filter((item) => item.admission === "admitted").length;
  const current = formationItems.filter((item) => item.catch_up === "current").length;
  const serving = formationItems.filter((item) => item.serving === "serving").length;
  const unknown = formationItems.filter((item) =>
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

function renderMetrics(data) {
  const values = [
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

function renderMembers(members) {
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

function renderPartitions(envelope, fallback) {
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
    ...distribution.slice(0, MAX_RENDERED_MEMBERS).map((item) =>
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

function renderClients(envelope) {
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
    ...protocols.map((protocol) =>
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

function renderNamespaces(envelope, cacheEnvelope) {
  const namespaces = envelope?.data?.items ?? [];
  byTest("namespace-table").replaceChildren(
    ...namespaces.slice(0, MAX_RENDERED_MEMBERS).map((namespace) =>
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
    ...caches.map((cache) =>
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

function renderFormation(envelope) {
  const items = envelope?.data?.items ?? [];
  byTest("formation-table").replaceChildren(
    ...items.map((item) =>
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

function renderConsensus(envelope, local) {
  const items = envelope?.data?.items ?? [];
  byTest("consensus-table").replaceChildren(
    ...items.map((item) => {
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

function renderRecovery(envelope) {
  const items = envelope?.data?.items ?? [];
  const outcomes = ["clean", "repaired", "partial", "corrupt", "failed"];
  byTest("recovery-outcomes").replaceChildren(
    ...outcomes.map((outcome) =>
      pill(
        outcome,
        items.filter((item) => item.outcome === outcome).length,
      ),
    ),
  );
  byTest("recovery-table").replaceChildren(
    ...items.map((item) =>
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

function renderPlacement(trace, fallback) {
  const placement = trace ?? fallback;
  const state = byTest("placement-state");
  state.textContent = placement?.outcome ?? "unavailable";
  state.className = `truth-chip ${placement?.outcome ?? "unavailable"}`;
  byTest("placement-details").replaceChildren(
    pill("selected", known(Array.isArray(placement?.selected) ? placement.selected.length : placement?.selected)),
    pill("rejected", known(trace ? trace.candidates?.items?.filter((item) => !item.selected).length : placement?.rejected)),
    pill("committed", known(trace?.commit_index ?? placement?.latest_committed_epoch)),
    pill("applied", known(trace?.applied_index ?? placement?.latest_applied_epoch)),
  );
  const candidates = trace?.candidates?.items ?? [];
  byTest("placement-table").replaceChildren(
    ...candidates.slice(0, MAX_RENDERED_MEMBERS).map((candidate) =>
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
  for (const svg of document.querySelectorAll("svg[data-series]")) {
    drawSparkline(svg, history.points(svg.dataset.series));
  }
}

function drawSparkline(svg, points) {
  svg.replaceChildren();
  const finite = points
    .map((point, index) => ({ x: index, y: point.value }))
    .filter((point) => point.y != null);
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

function renderDegraded(error) {
  badge("unavailable");
  setText("poll-state", "cannot reach management snapshots");
  const node = byTest("degraded-state");
  node.hidden = false;
  node.textContent = `Cannot reach cluster: ${error.message}`;
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

function badge(source) {
  const node = byTest("source-badge");
  node.textContent = source;
  node.dataset.source = source;
}
function metric(id, label, value, detail) {
  const node = byTest(id);
  node.replaceChildren();
  const a = document.createElement("span");
  a.textContent = label;
  const b = document.createElement("strong");
  b.textContent = value;
  const c = document.createElement("small");
  c.textContent = detail;
  node.append(a, b, c);
}
function pill(label, value) {
  const node = document.createElement("span");
  node.className = "metric-pill";
  const a = document.createElement("small");
  a.textContent = label;
  const b = document.createElement("strong");
  b.textContent = value;
  node.append(a, b);
  return node;
}
function fact(label, value) {
  const wrap = document.createElement("div");
  const dt = document.createElement("dt");
  dt.textContent = label;
  const dd = document.createElement("dd");
  dd.textContent = known(value);
  wrap.append(dt, dd);
  return wrap;
}
function row(values, data = {}) {
  const tr = document.createElement("tr");
  if (data.testid) tr.dataset.testid = data.testid;
  if (data.reachability) tr.dataset.reachability = data.reachability;
  tr.append(
    ...values.map((value) => {
      const td = document.createElement("td");
      if (value instanceof Node) td.append(value);
      else td.textContent = value ?? "unavailable";
      return td;
    }),
  );
  return tr;
}
function status(value) {
  return chip(value ?? "unknown", value ?? "unknown");
}
function chip(text, kind) {
  const node = document.createElement("span");
  node.className = `truth-chip ${kind}`;
  node.textContent = text;
  return node;
}
function byTest(id) {
  return document.querySelector(`[data-testid='${id}']`);
}
function setText(id, text) {
  byTest(id).textContent = text;
}
function known(value, suffix = "") {
  return value == null ? "unavailable" : `${new Intl.NumberFormat("en-US").format(value)}${suffix}`;
}
function format(value) {
  return known(value);
}
function truth(value) {
  return value === true ? "yes" : value === false ? "no" : "unknown";
}
function bytes(value) {
  if (value == null) return "unavailable";
  if (value < 1024) return `${value} B`;
  return `${(value / 1024).toFixed(1)} KiB`;
}
function duration(value) {
  if (value == null) return "unavailable";
  return value < 60 ? `${value}s` : `${Math.floor(value / 60)}m`;
}
function boundedCount(value) {
  return value?.value == null ? "unavailable" : `${value.value}${value.exact ? "" : "+"}`;
}
function recoveryLabel(items) {
  if (items.length === 0) return "source unavailable";
  const bad = items.filter((item) => ["corrupt", "failed", "partial"].includes(item.outcome)).length;
  return bad ? `${bad} need attention` : "no adverse outcome observed";
}
