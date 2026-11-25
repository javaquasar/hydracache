// Unit tests for the URL/share helpers (node:test, no browser).
import assert from "node:assert/strict";
import { describe, test } from "node:test";

import {
  MAX_SEED,
  decodeReplayScript,
  encodeReplayScript,
  parseSeed,
  readInitialState,
  reproducerCommand,
  snapshotHash,
  validateApiBase,
  writeUrlState,
} from "../share.js";

const ORIGIN = "https://javaquasar.github.io";

function fakeHistory() {
  return {
    url: null,
    replaceState(_state, _title, url) {
      this.url = url;
    },
    params() {
      return new URLSearchParams(this.url.slice(1));
    },
  };
}

describe("readInitialState", () => {
  test("defaults to a plain seeded wasm run", () => {
    const initial = readInitialState("", ORIGIN);
    assert.equal(initial.seed, 80);
    assert.equal(initial.steps, 0);
    assert.equal(initial.scenario, "default");
    assert.equal(initial.engine, "wasm");
    assert.equal(initial.script, null);
    assert.deepEqual(initial.warnings, []);
  });

  test("reports unusable parameters instead of silently replacing them", () => {
    const initial = readInitialState("?seed=abc&steps=-1&script=%%%", ORIGIN);
    assert.equal(initial.seed, 80);
    assert.equal(initial.steps, 0);
    assert.equal(initial.warnings.length, 3);
    assert.match(initial.warnings.join(" "), /URL seed "abc"/);
    assert.match(initial.warnings.join(" "), /URL steps "-1"/);
    assert.match(initial.warnings.join(" "), /replay script is invalid/);
  });

  test("rejects seeds JSON cannot round-trip exactly", () => {
    const initial = readInitialState(`?seed=${MAX_SEED + 2}`, ORIGIN);
    assert.equal(initial.seed, 80);
    assert.equal(initial.warnings.length, 1);
  });

  test("a replay script wins over seed and scenario parameters", () => {
    const script = encodeReplayScript({
      version: 1,
      seed: 9,
      mode: "mixed",
      scenario: "minority_partition_cannot_commit",
      actions: [],
      target_step: 12,
    });
    const initial = readInitialState(`?seed=1&scenario=default&script=${script}`, ORIGIN);
    assert.equal(initial.seed, 9);
    assert.equal(initial.scenario, "minority_partition_cannot_commit");
    assert.equal(initial.mode, "mixed");
    assert.equal(initial.script.target_step, 12);
  });

  test("legacy links without target_step resume at their steps parameter", () => {
    const script = encodeReplayScript({ version: 1, seed: 3, mode: "manual", actions: [] });
    const initial = readInitialState(`?steps=21&script=${script}`, ORIGIN);
    assert.equal(initial.script.target_step, 21);
  });

  test("a refused server API is reported and not used", () => {
    const initial = readInitialState("?engine=server&api=https://evil.example", ORIGIN);
    assert.equal(initial.engine, "server");
    assert.equal(initial.apiBase, "");
    assert.match(initial.apiError, /was refused/);
  });
});

describe("validateApiBase", () => {
  for (const [value, allowed] of [
    ["", true],
    ["/api", true],
    ["http://127.0.0.1:3000", true],
    ["http://localhost:3000/", true],
    ["http://[::1]:3000", true],
    [`${ORIGIN}/sim`, true],
    ["//evil.example/api", false],
    ["https://evil.example", false],
    ["https://127.0.0.1.evil.example", false],
    ["javascript:alert(1)", false],
    ["ftp://127.0.0.1", false],
    ["not a url", false],
  ]) {
    test(`${value || "(empty)"} is ${allowed ? "allowed" : "refused"}`, () => {
      const result = validateApiBase(value, ORIGIN);
      assert.equal(result.error === "", allowed, JSON.stringify(result));
      if (!allowed) {
        assert.equal(result.apiBase, "");
      }
    });
  }
});

describe("parseSeed", () => {
  test("accepts whole numbers in the exact JSON range only", () => {
    assert.equal(parseSeed(" 42 "), 42);
    assert.equal(parseSeed("0"), 0);
    assert.equal(parseSeed(String(MAX_SEED)), MAX_SEED);
    for (const bad of ["", "-1", "1.5", "1e3", "0x10", "12abc", String(MAX_SEED + 2)]) {
      assert.equal(parseSeed(bad), null, bad);
    }
  });
});

describe("writeUrlState", () => {
  const snapshot = { seed: 5, step: 17 };

  test("the guide step is shared from step 2 on and read back", () => {
    const history = fakeHistory();
    writeUrlState(history, snapshot, "default", "wasm", "", "", 4);
    assert.equal(history.params().get("tourStep"), "4");
    assert.equal(readInitialState(`?${history.params().toString()}`, ORIGIN).tourStep, 4);
    const first = fakeHistory();
    writeUrlState(first, snapshot, "default", "wasm", "", "", 1);
    assert.equal(first.params().get("tourStep"), null);
    assert.equal(readInitialState("?seed=5", ORIGIN).tourStep, 0);
  });

  test("a plain run shares only seed, steps and scenario", () => {
    const history = fakeHistory();
    writeUrlState(history, snapshot, "default", "wasm", "", JSON.stringify({
      version: 1,
      seed: 5,
      mode: "manual",
      scenario: null,
      actions: [],
    }));
    const params = history.params();
    assert.equal(params.get("seed"), "5");
    assert.equal(params.get("steps"), "17");
    assert.equal(params.get("script"), null);
    assert.equal(params.get("engine"), null);
  });

  test("an intervened run shares the script resumed at the visible step", () => {
    const history = fakeHistory();
    writeUrlState(history, snapshot, "default", "server", "http://127.0.0.1:3000/", JSON.stringify({
      version: 1,
      seed: 5,
      mode: "manual",
      scenario: null,
      actions: [{ type: "crash", at_step: 4, node: "node-0" }],
    }));
    const params = history.params();
    assert.equal(params.get("engine"), "server");
    assert.equal(params.get("api"), "http://127.0.0.1:3000");
    const script = decodeReplayScript(params.get("script"));
    assert.equal(script.target_step, 17);
    assert.equal(script.actions[0].type, "crash");
  });
});

describe("reproducerCommand", () => {
  test("a plain run gets the vopr seed command", () => {
    assert.equal(
      reproducerCommand(5, 17),
      "cargo run -p hydracache-sim --bin vopr -- --seed 5 --steps 17",
    );
  });

  test("an intervened run gets a replay link pinned to the step", () => {
    const link = reproducerCommand(
      5,
      17,
      JSON.stringify({ version: 1, seed: 5, mode: "mixed", scenario: null, actions: [] }),
      `${ORIGIN}/hydracache/demo/?seed=5&steps=3`,
    );
    const url = new URL(link);
    assert.equal(url.pathname, "/hydracache/demo/");
    assert.deepEqual([...url.searchParams.keys()], ["script"]);
    assert.equal(decodeReplayScript(url.searchParams.get("script")).target_step, 17);
  });

  test("a server-engine replay link keeps the engine and API", () => {
    const link = reproducerCommand(
      5,
      17,
      JSON.stringify({ version: 1, seed: 5, mode: "manual", scenario: null, actions: [{ type: "crash", at_step: 2, node: "node-0" }] }),
      "http://127.0.0.1:5173/demo/?seed=5&engine=server&api=http%3A%2F%2F127.0.0.1%3A3000%2F",
    );
    const url = new URL(link);
    assert.equal(url.searchParams.get("engine"), "server");
    assert.equal(url.searchParams.get("api"), "http://127.0.0.1:3000");
    assert.ok(url.searchParams.get("script"));
  });
});

describe("replay script encoding", () => {
  test("round-trips non-ASCII payloads", () => {
    const script = {
      version: 1,
      seed: 1,
      mode: "manual",
      scenario: null,
      actions: [{ type: "push_event", at_step: 0, client: "клиент", ns: "профили", key: "k", value: "✓" }],
      target_step: 2,
    };
    assert.deepEqual(decodeReplayScript(encodeReplayScript(script)), script);
  });

  test("rejects future versions, oversized logs and bad target steps", () => {
    const base = { version: 1, seed: 1, mode: "manual", scenario: null, actions: [] };
    assert.equal(decodeReplayScript(encodeReplayScript({ ...base, version: 2 })), null);
    assert.equal(
      decodeReplayScript(encodeReplayScript({ ...base, actions: new Array(257).fill({ type: "step" }) })),
      null,
    );
    assert.equal(decodeReplayScript(encodeReplayScript({ ...base, target_step: -1 })), null);
  });
});

describe("snapshotHash", () => {
  test("is independent of key order", () => {
    assert.equal(snapshotHash({ a: 1, b: { c: 2, d: [1, 2] } }), snapshotHash({ b: { d: [1, 2], c: 2 }, a: 1 }));
    assert.notEqual(snapshotHash({ a: 1 }), snapshotHash({ a: 2 }));
  });
});
