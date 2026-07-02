const MAX_RENDERED_MEMBERS = 48;
const POLL_INTERVAL_MS = 10_000;
const OVERVIEW_URL = "/cluster/overview";
const METRICS_URL = "/metrics";

const els = {};

document.addEventListener("DOMContentLoaded", () => {
  cacheElements();
  loadSnapshot();
  window.setInterval(loadSnapshot, POLL_INTERVAL_MS);
});

function cacheElements() {
  els.sourceBadge = document.querySelector("[data-testid='source-badge']");
  els.pollState = document.querySelector("[data-testid='poll-state']");
  els.degraded = document.querySelector("[data-testid='degraded-state']");
  els.leader = document.querySelector("[data-testid='leader']");
  els.partitions = document.querySelector("[data-testid='partition-summary']");
  els.backupAge = document.querySelector("[data-testid='backup-age']");
  els.lifecycle = document.querySelector("[data-testid='lifecycle-panel']");
  els.renderCap = document.querySelector("[data-testid='render-cap']");
  els.graph = document.querySelector("[data-testid='topology-graph']");
  els.membersList = document.querySelector("[data-testid='members-list']");
  els.consistencyDefault = document.querySelector("[data-field='configured-default']");
  els.consistencyCounts = document.querySelector("[data-field='consistency-counts']");
  els.metricsStrip = document.querySelector("[data-testid='metrics-strip']");
}

async function loadSnapshot() {
  try {
    setPollState("refreshing admin view");
    const [overview, metricsText] = await Promise.all([fetchOverview(), fetchMetrics()]);
    renderOverview(overview, parseMetrics(metricsText));
  } catch (error) {
    renderDegraded(error);
  }
}

async function fetchOverview() {
  const response = await fetch(OVERVIEW_URL, { cache: "no-store" });
  if (!response.ok) {
    throw new Error(`cluster overview returned ${response.status}`);
  }
  return response.json();
}

async function fetchMetrics() {
  try {
    const response = await fetch(METRICS_URL, { cache: "no-store" });
    return response.ok ? response.text() : "";
  } catch (_error) {
    return "";
  }
}

function renderOverview(overview, metrics) {
  const source = overview?.source === "live" ? "live" : "modeled";
  els.degraded.hidden = true;
  els.degraded.textContent = "";
  els.sourceBadge.textContent = source;
  els.sourceBadge.dataset.source = source;
  setPollState(`last refresh ${new Date().toLocaleTimeString()}`);

  renderLeader(overview);
  renderPartitions(overview);
  renderBackup(overview);
  renderLifecycle(overview);
  renderMembers(overview?.members ?? [], overview?.leader?.node_id ?? null);
  renderConsistency(overview?.consistency);
  renderMetrics(metrics);
}

function renderDegraded(error) {
  els.sourceBadge.textContent = "unreachable";
  els.sourceBadge.dataset.source = "unreachable";
  setPollState("cannot reach cluster");
  els.degraded.hidden = false;
  els.degraded.textContent = `Cannot reach cluster: ${error.message}`;
  setMetricCell(els.leader, "Leader", "unavailable", "no trusted snapshot");
  setMetricCell(els.partitions, "Partitions", "unavailable", "no trusted snapshot");
  setMetricCell(els.backupAge, "Backup age", "unavailable", "no trusted snapshot");
  setMetricCell(els.lifecycle, "Lifecycle", "unavailable", "no trusted snapshot");
  els.renderCap.textContent = "no members rendered";
  els.membersList.replaceChildren();
  els.graph.replaceChildren();
  els.consistencyDefault.textContent = "-";
  els.consistencyCounts.textContent = "-";
  els.metricsStrip.replaceChildren(textNode("metrics unavailable"));
}

function setPollState(text) {
  els.pollState.textContent = text;
}

function renderLeader(overview) {
  const leader = overview?.leader;
  if (!leader) {
    setMetricCell(els.leader, "Leader", "electing", "term - / epoch -");
    return;
  }
  setMetricCell(els.leader, "Leader", leader.node_id, `term ${leader.term} / epoch ${leader.epoch}`);
}

function renderPartitions(overview) {
  const partitions = overview?.partitions ?? { count: 0, under_replicated: 0 };
  setMetricCell(
    els.partitions,
    "Partitions",
    formatNumber(partitions.count),
    `under-replicated ${formatNumber(partitions.under_replicated)}`,
  );
}

function renderBackup(overview) {
  const seconds = overview?.backup_age_seconds;
  setMetricCell(
    els.backupAge,
    "Backup age",
    seconds == null ? "none" : formatDuration(seconds),
    seconds == null ? "no snapshot recorded" : "worst known namespace",
  );
}

function renderLifecycle(overview) {
  const lifecycle = overview?.lifecycle ?? { reshard_phase: "idle", upgrade_phase: "idle" };
  setMetricCell(
    els.lifecycle,
    "Lifecycle",
    lifecycle.reshard_phase,
    `upgrade ${lifecycle.upgrade_phase}`,
  );
}

function setMetricCell(cell, label, value, detail) {
  cell.replaceChildren();
  const span = document.createElement("span");
  span.textContent = label;
  const strong = document.createElement("strong");
  strong.textContent = value;
  const small = document.createElement("small");
  small.textContent = detail;
  cell.append(span, strong, small);
}

function renderMembers(members, leaderId) {
  const rendered = members.slice(0, MAX_RENDERED_MEMBERS);
  const hidden = Math.max(0, members.length - rendered.length);
  els.renderCap.textContent =
    hidden > 0
      ? `${rendered.length} rendered, ${hidden} not rendered`
      : `${rendered.length} rendered`;

  els.membersList.replaceChildren(...rendered.map((member) => memberCard(member, leaderId)));
  renderGraph(rendered, leaderId, members.length);
}

function memberCard(member, leaderId) {
  const card = document.createElement("article");
  card.className = "member-card";
  card.dataset.testid = "member";
  card.dataset.reachability = member.reachability ?? (member.reachable ? "reachable" : "unreachable");
  if (member.node_id === leaderId) {
    card.dataset.leader = "true";
  }

  const top = document.createElement("div");
  top.className = "member-topline";
  const id = document.createElement("strong");
  id.textContent = member.node_id;
  const role = document.createElement("span");
  role.textContent = member.role ?? "member";
  top.append(id, role);

  const detail = document.createElement("p");
  detail.textContent = `${member.reachability ?? "unknown"} / generation ${member.generation ?? 0}`;
  card.append(top, detail);
  return card;
}

function renderGraph(members, leaderId, totalMembers) {
  els.graph.replaceChildren();
  const width = 640;
  const height = 360;
  const center = { x: width / 2, y: height / 2 };
  const radius = members.length <= 8 ? 122 : 142;
  const positions = new Map();

  members.forEach((member, index) => {
    const angle = members.length === 1 ? 0 : (Math.PI * 2 * index) / members.length - Math.PI / 2;
    positions.set(member.node_id, {
      x: members.length === 1 ? center.x : center.x + Math.cos(angle) * radius,
      y: members.length === 1 ? center.y : center.y + Math.sin(angle) * radius,
    });
  });

  if (leaderId && positions.has(leaderId)) {
    const leaderPosition = positions.get(leaderId);
    for (const [nodeId, position] of positions) {
      if (nodeId === leaderId) {
        continue;
      }
      els.graph.append(svgLine(leaderPosition, position));
    }
  }

  for (const member of members) {
    const position = positions.get(member.node_id);
    els.graph.append(svgNode(member, position, member.node_id === leaderId, totalMembers));
  }
}

function svgLine(from, to) {
  const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
  line.setAttribute("class", "graph-link");
  line.setAttribute("x1", from.x.toFixed(1));
  line.setAttribute("y1", from.y.toFixed(1));
  line.setAttribute("x2", to.x.toFixed(1));
  line.setAttribute("y2", to.y.toFixed(1));
  return line;
}

function svgNode(member, position, isLeader, totalMembers) {
  const group = document.createElementNS("http://www.w3.org/2000/svg", "g");
  group.setAttribute("class", `graph-node ${isLeader ? "leader" : ""}`);
  group.setAttribute("transform", `translate(${position.x.toFixed(1)} ${position.y.toFixed(1)})`);
  const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");
  circle.setAttribute("r", isLeader ? "18" : "14");
  circle.setAttribute("data-reachability", member.reachability ?? "unknown");
  const title = document.createElementNS("http://www.w3.org/2000/svg", "title");
  title.textContent = `${member.node_id} ${member.reachability ?? "unknown"}`;
  group.append(title, circle);
  if (totalMembers <= 16) {
    const label = document.createElementNS("http://www.w3.org/2000/svg", "text");
    label.setAttribute("y", "33");
    label.textContent = truncateMiddle(member.node_id, 14);
    group.append(label);
  }
  return group;
}

function renderConsistency(consistency) {
  els.consistencyDefault.textContent = consistency?.configured_default ?? "not configured";
  const counts = consistency?.op_counts_by_level ?? [];
  els.consistencyCounts.textContent =
    counts.length === 0
      ? "no operation counts"
      : counts.map((entry) => `${entry.level}: ${formatNumber(entry.count)}`).join(", ");
}

function renderMetrics(metrics) {
  els.metricsStrip.replaceChildren(
    metricPill("hit ratio", metrics.hitRatio == null ? "-" : formatRatio(metrics.hitRatio)),
    metricPill("admission rejects", formatNumber(metrics.admissionRejected ?? 0)),
    metricPill("queue depth", formatNumber(metrics.queueDepth ?? 0)),
    metricPill("members gauge", formatNumber(metrics.clusterMembers ?? 0)),
  );
}

function metricPill(label, value) {
  const pill = document.createElement("span");
  pill.className = "metric-pill";
  const labelNode = document.createElement("small");
  labelNode.textContent = label;
  const valueNode = document.createElement("strong");
  valueNode.textContent = value;
  pill.append(labelNode, valueNode);
  return pill;
}

function parseMetrics(text) {
  const metrics = {
    hitRatio: null,
    admissionRejected: null,
    queueDepth: null,
    clusterMembers: null,
  };
  for (const line of text.split("\n")) {
    if (!line || line.startsWith("#")) {
      continue;
    }
    const match = line.match(/^([a-zA-Z_:][a-zA-Z0-9_:]*)(?:\{[^}]*\})?\s+(-?\d+(?:\.\d+)?)/);
    if (!match) {
      continue;
    }
    const value = Number.parseFloat(match[2]);
    switch (match[1]) {
      case "hydracache_cache_hit_ratio":
        metrics.hitRatio ??= value;
        break;
      case "hydracache_admission_rejected_total":
        metrics.admissionRejected = value;
        break;
      case "hydracache_admission_queue_depth":
        metrics.queueDepth = value;
        break;
      case "hydracache_cluster_members":
        metrics.clusterMembers = value;
        break;
    }
  }
  return metrics;
}

function textNode(text) {
  return document.createTextNode(text);
}

function formatNumber(value) {
  return new Intl.NumberFormat("en-US").format(value);
}

function formatRatio(value) {
  return `${Math.round(value * 1000) / 10}%`;
}

function formatDuration(seconds) {
  if (seconds < 60) {
    return `${seconds}s`;
  }
  if (seconds < 3600) {
    return `${Math.floor(seconds / 60)}m`;
  }
  return `${Math.floor(seconds / 3600)}h`;
}

function truncateMiddle(value, maxLength) {
  if (value.length <= maxLength) {
    return value;
  }
  return `${value.slice(0, 6)}...${value.slice(-5)}`;
}
