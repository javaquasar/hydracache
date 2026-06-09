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

  compareBenchmarks(baseline, candidate) {
    return this.post("/demo/benchmarks/compare", { baseline, candidate });
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
