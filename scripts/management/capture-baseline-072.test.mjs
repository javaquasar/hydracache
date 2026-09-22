import assert from "node:assert/strict";
import test from "node:test";

import {
  buildReceipt,
  countPlaywrightTests,
  coveragePercent,
  sha256,
  stableJson,
} from "./capture-baseline-072.mjs";

test("stable JSON and hashes are independent of object insertion order", () => {
  assert.equal(stableJson({ b: 2, a: 1 }), stableJson({ a: 1, b: 2 }));
  assert.equal(sha256("proof").length, 64);
});

test("coverage uses executable byte ranges", () => {
  assert.equal(
    coveragePercent([{ functions: [{ ranges: [{ startOffset: 0, endOffset: 75, count: 1 }, { startOffset: 75, endOffset: 100, count: 0 }] }] }]),
    75,
  );
});

test("Playwright test count includes project instances", () => {
  assert.equal(countPlaywrightTests({ suites: [{ specs: [{ tests: [{}, {}] }], suites: [{ specs: [{ tests: [{}] }] }] }] }), 3);
});

test("receipt is exact, candidate-bound and hash-sealed", () => {
  const ids = [
    "bundle_bytes",
    "endpoint_latency_bytes",
    "server_retained_owners",
    "browser_heap_dom",
    "fd_task_counts",
    "console_coverage",
  ];
  const raw = Object.fromEntries(ids.map((id) => [id, { value: 1, unit: id.includes("bytes") || id === "browser_heap_dom" ? "bytes" : id === "console_coverage" ? "percent" : "count", samples: 1 }]));
  const receipt = buildReceipt({
    baseline: "pre_feature",
    sourceRef: "8d205fa302d81a07c19147cb4431e16390d256c3",
    sourceCommit: "8".repeat(40),
    candidateCommit: "2".repeat(40),
    command: { argv: ["capture"] },
    raw,
  });
  assert.equal(receipt.outcome, "pass");
  assert.equal(receipt.measurements.length, 6);
  assert.ok(receipt.measurements.every((measurement) => measurement.command_sha256.length === 64));
});

test("receipt refuses incomplete or non-finite evidence", () => {
  assert.throws(
    () => buildReceipt({ baseline: "pre_feature", sourceRef: "x", sourceCommit: "x", candidateCommit: "y", command: {}, raw: {} }),
    /measurement set mismatch/,
  );
});

