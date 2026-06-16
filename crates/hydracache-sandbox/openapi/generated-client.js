// Minimal fetch client generated from the sandbox OpenAPI contract examples.
// It intentionally has no dependencies, so it can be pasted into a browser
// console while the sandbox is running at http://127.0.0.1:3000.

export class HydraCacheSandboxClient {
  constructor(baseUrl = "http://127.0.0.1:3000", token = null) {
    this.baseUrl = baseUrl.replace(/\/$/, "");
    this.token = token;
  }

  headers() {
    const headers = { "content-type": "application/json" };
    if (this.token) {
      headers.authorization = `Bearer ${this.token}`;
    }
    return headers;
  }

  async json(path) {
    const response = await fetch(`${this.baseUrl}${path}`, {
      headers: this.headers()
    });
    return response.json();
  }

  async post(path, body = null) {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method: "POST",
      headers: this.headers(),
      body: body ? JSON.stringify(body) : null
    });
    return response.json();
  }

  ready() {
    return this.json("/ready");
  }

  runScenarioDocument(document) {
    return this.post("/demo/scenarios/document/run", document);
  }

  runScenarioFile(path = "golden-path.yaml", format = "yaml") {
    return this.post("/demo/scenarios/file/run", { path, format });
  }

  scenarioCatalog() {
    return this.json("/demo/scenarios/catalog");
  }

  runScenarioSuiteFile(path = "regression-suite.json") {
    return this.post("/demo/scenarios/suite/file/run", { path });
  }

  eventSummary() {
    return this.json("/demo/events/summary");
  }

  runEventPreflight(options = {}) {
    return this.post("/demo/events/preflight/run", options);
  }

  rolloutCompare(options = {}) {
    return this.post("/demo/rollout/compare", options);
  }

  runDbSoak(options = {}) {
    return this.post("/demo/db/soak/run", options);
  }

  compareBenchmarks(baseline, candidate) {
    return this.post("/demo/benchmarks/compare", { baseline, candidate });
  }

  flows() {
    return this.json("/demo/flows");
  }

  replayFlow(flowId, body = {}) {
    return this.post(`/demo/flows/${encodeURIComponent(flowId)}/replay`, body);
  }

  loadProduct(id, options = {}) {
    return this.post(`/demo/query/products/${id}/load`, options);
  }

  loadOrderSummary(id, options = {}) {
    return this.post(`/demo/query/orders/${id}/summary/load`, options);
  }

  runClusterOwnership(options = {}) {
    return this.post("/demo/cluster/ownership/run", options);
  }

  runClusterOwnershipTransfer(options = {}) {
    return this.post("/demo/cluster/ownership-transfer/run", options);
  }

  runClusterRoutedPeerFetch(options = {}) {
    return this.post("/demo/cluster/routed-peer-fetch/run", options);
  }

  runClusterReadThrough(options = {}) {
    return this.post("/demo/cluster/read-through/run", options);
  }

  runClusterOwnerLoad(options = {}) {
    return this.post("/demo/cluster/owner-load/run", options);
  }

  runRealClusterAdapters(options = {}) {
    return this.post("/demo/cluster/real-adapters/run", options);
  }

  exportSession() {
    return this.json("/demo/export");
  }

  importSession(bundle, source = "generated-client") {
    return this.post("/demo/import", {
      replace_events: true,
      source,
      bundle
    });
  }
}
