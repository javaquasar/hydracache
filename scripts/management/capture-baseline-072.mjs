#!/usr/bin/env node

import { createHash } from "node:crypto";
import { execFileSync, spawn } from "node:child_process";
import { createRequire } from "node:module";
import { readdirSync, readFileSync, realpathSync, statSync, writeFileSync, mkdirSync } from "node:fs";
import { request } from "node:http";
import { basename, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const SCRIPT_PATH = fileURLToPath(import.meta.url);

const BASELINES = {
  pre_feature: {
    sourceRef: "8d205fa302d81a07c19147cb4431e16390d256c3",
    measurements: [
      "bundle_bytes",
      "endpoint_latency_bytes",
      "server_retained_owners",
      "browser_heap_dom",
      "fd_task_counts",
      "console_coverage",
    ],
  },
  published_previous: {
    sourceRef: "v0.71.0",
    measurements: [
      "legacy_console_tests",
      "bundle_bytes",
      "endpoint_latency_bytes",
      "server_retained_owners",
      "browser_heap_dom",
      "fd_task_counts",
      "console_coverage",
    ],
  },
};

export function stableJson(value) {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

export function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

export function buildReceipt({ baseline, sourceRef, sourceCommit, candidateCommit, command, raw }) {
  const required = BASELINES[baseline]?.measurements;
  if (!required) throw new Error(`unsupported baseline: ${baseline}`);
  const observed = Object.keys(raw).sort();
  if (stableJson(observed) !== stableJson([...required].sort())) {
    throw new Error(`measurement set mismatch: expected ${required.join(", ")}; got ${observed.join(", ")}`);
  }
  const commandSha = sha256(stableJson(command));
  const measurements = required.map((id) => {
    const result = raw[id];
    if (!Number.isFinite(result.value) || result.value < 0 || !Number.isInteger(result.samples) || result.samples < 1) {
      throw new Error(`${id} has invalid value or sample count`);
    }
    if (!["bytes", "milliseconds", "count", "percent"].includes(result.unit)) {
      throw new Error(`${id} has unsupported unit ${result.unit}`);
    }
    return {
      id,
      value: result.value,
      unit: result.unit,
      samples: result.samples,
      command_sha256: commandSha,
      output_sha256: sha256(stableJson(result)),
    };
  });
  return {
    schema_version: 1,
    release: "0.72.0",
    baseline,
    source_ref: sourceRef,
    source_commit: sourceCommit,
    candidate_commit: candidateCommit,
    dirty_worktree: false,
    outcome: "pass",
    measurements,
  };
}

export function coveragePercent(entries) {
  let total = 0;
  let used = 0;
  for (const entry of entries) {
    for (const fn of entry.functions ?? []) {
      for (const range of fn.ranges ?? []) {
        const bytes = Math.max(0, range.endOffset - range.startOffset);
        total += bytes;
        if (range.count > 0) used += bytes;
      }
    }
  }
  return total === 0 ? 0 : Number(((used / total) * 100).toFixed(4));
}

export function countPlaywrightTests(report) {
  let count = 0;
  const visit = (suite) => {
    for (const spec of suite.specs ?? []) count += (spec.tests ?? []).length;
    for (const nested of suite.suites ?? []) visit(nested);
  };
  for (const suite of report.suites ?? []) visit(suite);
  return count;
}

function parseArgs(argv) {
  const values = {};
  for (let index = 0; index < argv.length; index += 2) {
    const key = argv[index];
    const value = argv[index + 1];
    if (!key?.startsWith("--") || !value) throw new Error(`invalid argument near ${key ?? "<end>"}`);
    values[key.slice(2)] = value;
  }
  for (const required of ["baseline", "source-root", "candidate-root", "output"]) {
    if (!values[required]) throw new Error(`--${required} is required`);
  }
  return values;
}

function git(root, ...args) {
  return execFileSync("git", ["-C", root, ...args], { encoding: "utf8" }).trim();
}

function executable(name) {
  return process.platform === "win32" ? `${name}.cmd` : name;
}

function run(program, args, cwd, options = {}) {
  try {
    return execFileSync(program, args, {
      cwd,
      encoding: "utf8",
      stdio: options.capture ? ["ignore", "pipe", "pipe"] : "inherit",
      env: { ...process.env, ...options.env },
      maxBuffer: 64 * 1024 * 1024,
    });
  } catch (error) {
    const stdout = error.stdout?.toString().trim();
    const stderr = error.stderr?.toString().trim();
    throw new Error(
      [`command failed: ${program} ${args.join(" ")}`, stdout && `stdout:\n${stdout}`, stderr && `stderr:\n${stderr}`]
        .filter(Boolean)
        .join("\n"),
      { cause: error },
    );
  }
}

function bundleBytes(consoleRoot) {
  return readdirSync(consoleRoot)
    .filter((name) => [".html", ".js", ".css"].some((extension) => name.endsWith(extension)))
    .reduce((sum, name) => sum + statSync(join(consoleRoot, name)).size, 0);
}

function processCounts(pid) {
  if (process.platform !== "linux") throw new Error("baseline resource capture requires Linux /proc");
  return {
    fd: readdirSync(`/proc/${pid}/fd`).length,
    tasks: readdirSync(`/proc/${pid}/task`).length,
  };
}

function waitForHttp(url, timeoutMs = 30_000) {
  const deadline = Date.now() + timeoutMs;
  return new Promise((resolvePromise, rejectPromise) => {
    const attempt = () => {
      const req = request(url, (response) => {
        response.resume();
        if ((response.statusCode ?? 500) < 500) resolvePromise();
        else retry();
      });
      req.on("error", retry);
      req.setTimeout(1_000, () => req.destroy());
    };
    const retry = () => {
      if (Date.now() >= deadline) rejectPromise(new Error(`server did not become ready: ${url}`));
      else setTimeout(attempt, 200);
    };
    attempt();
  });
}

async function captureBrowser(consoleRoot, port) {
  const requireFromConsole = createRequire(join(consoleRoot, "package.json"));
  const { chromium } = requireFromConsole("@playwright/test");
  const fixtures = await import(pathToFileURL(join(consoleRoot, "tests", "fixtures.js")));
  const overviewBody = JSON.stringify(fixtures.liveOverviewFixture);
  const metricsBody = fixtures.metricsFixture;
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  await page.coverage.startJSCoverage({ resetOnNavigation: false });
  await page.route("**/cluster/overview", (route) => route.fulfill({ contentType: "application/json", body: overviewBody }));
  await page.route("**/metrics", (route) => route.fulfill({ contentType: "text/plain", body: metricsBody }));
  const latencies = [];
  for (let index = 0; index < 5; index += 1) {
    const started = performance.now();
    await page.goto(`http://127.0.0.1:${port}/console/`, { waitUntil: "networkidle" });
    await page.locator("[data-testid='source-badge']").waitFor();
    latencies.push(performance.now() - started);
  }
  const domNodes = await page.locator("*").count();
  const heapBytes = await page.evaluate(() => performance.memory?.usedJSHeapSize ?? 0);
  const coverage = await page.coverage.stopJSCoverage();
  await browser.close();
  if (domNodes < 1 || heapBytes < 1) throw new Error("Chromium did not expose non-empty heap and DOM measurements");
  const measuredCoverage = coveragePercent(coverage);
  if (measuredCoverage <= 0) throw new Error("Playwright JavaScript coverage is empty");
  return {
    latencyMs: Number((latencies.reduce((sum, value) => sum + value, 0) / latencies.length).toFixed(4)),
    samples: latencies.length,
    responseBytes: Buffer.byteLength(overviewBody) + Buffer.byteLength(metricsBody),
    domNodes,
    heapBytes,
    coveragePercent: measuredCoverage,
  };
}

async function capture(options) {
  const definition = BASELINES[options.baseline];
  if (!definition) throw new Error(`unsupported baseline: ${options.baseline}`);
  const sourceRoot = realpathSync(resolve(options["source-root"]));
  const candidateRoot = realpathSync(resolve(options["candidate-root"]));
  const output = resolve(options.output);
  const sourceCommit = git(sourceRoot, "rev-parse", "HEAD");
  const expectedSource = git(candidateRoot, "rev-parse", `${definition.sourceRef}^{commit}`);
  if (sourceCommit !== expectedSource) throw new Error(`source checkout is ${sourceCommit}, expected ${expectedSource}`);
  const candidateCommit = git(candidateRoot, "rev-parse", "HEAD");
  if (git(candidateRoot, "status", "--porcelain", "--untracked-files=normal")) {
    throw new Error("candidate worktree must be clean");
  }
  if (git(sourceRoot, "status", "--porcelain", "--untracked-files=normal")) {
    throw new Error("source baseline worktree must be clean");
  }

  const consoleRoot = join(sourceRoot, "console");
  run(executable("npm"), ["ci"], consoleRoot);
  run(executable("npm"), ["run", "build"], consoleRoot);
  const testJson = run(executable("npx"), ["playwright", "test", "--reporter=json"], consoleRoot, {
    capture: true,
    env: {
      CI: "1",
      HYDRACACHE_CONSOLE_PORT: "55171",
      HYDRACACHE_CONSOLE_URL: "http://127.0.0.1:55171/console/",
    },
  });
  const testReport = JSON.parse(testJson);
  const failed = testReport.stats?.unexpected ?? 0;
  if (failed !== 0) throw new Error(`legacy console tests failed: ${failed}`);

  const port = 55172;
  const server = spawn(process.execPath, [join(consoleRoot, "scripts", "serve-static.mjs")], {
    cwd: consoleRoot,
    env: { ...process.env, HYDRACACHE_CONSOLE_PORT: String(port) },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let serverStdout = "";
  let serverStderr = "";
  server.stdout.on("data", (chunk) => {
    serverStdout += chunk.toString();
  });
  server.stderr.on("data", (chunk) => {
    serverStderr += chunk.toString();
  });
  try {
    try {
      await waitForHttp(`http://127.0.0.1:${port}/console/`);
    } catch (error) {
      throw new Error(
        `${error.message}; origin exit=${server.exitCode ?? "running"}; stdout=${serverStdout.trim() || "<empty>"}; stderr=${serverStderr.trim() || "<empty>"}`,
        { cause: error },
      );
    }
    const before = processCounts(server.pid);
    const browser = await captureBrowser(consoleRoot, port);
    await new Promise((resolvePromise) => setTimeout(resolvePromise, 1_000));
    const after = processCounts(server.pid);
    const retained = Math.max(0, after.fd - before.fd) + Math.max(0, after.tasks - before.tasks);
    const raw = {
      bundle_bytes: { value: bundleBytes(consoleRoot), unit: "bytes", samples: 1 },
      endpoint_latency_bytes: {
        value: browser.latencyMs,
        unit: "milliseconds",
        samples: browser.samples,
        response_bytes: browser.responseBytes,
      },
      server_retained_owners: {
        value: retained,
        unit: "count",
        samples: browser.samples,
        before,
        after,
      },
      browser_heap_dom: {
        value: browser.heapBytes,
        unit: "bytes",
        samples: 1,
        dom_nodes: browser.domNodes,
      },
      fd_task_counts: {
        value: after.fd + after.tasks,
        unit: "count",
        samples: 1,
        ...after,
      },
      console_coverage: { value: browser.coveragePercent, unit: "percent", samples: 1 },
    };
    if (options.baseline === "published_previous") {
      raw.legacy_console_tests = { value: countPlaywrightTests(testReport), unit: "count", samples: 1 };
    }
    const command = {
      tool: basename(SCRIPT_PATH),
      tool_sha256: sha256(readFileSync(new URL(import.meta.url))),
      baseline: options.baseline,
      source_ref: definition.sourceRef,
      source_commit: sourceCommit,
      candidate_commit: candidateCommit,
      node: process.version,
      platform: `${process.platform}-${process.arch}`,
    };
    const receipt = buildReceipt({
      baseline: options.baseline,
      sourceRef: definition.sourceRef,
      sourceCommit,
      candidateCommit,
      command,
      raw,
    });
    mkdirSync(resolve(output, ".."), { recursive: true });
    writeFileSync(output, `${JSON.stringify(receipt, null, 2)}\n`);
    writeFileSync(`${output}.raw.json`, `${JSON.stringify({ command, measurements: raw }, null, 2)}\n`);
    console.log(`management-baseline-072: wrote ${output}`);
  } finally {
    server.kill("SIGTERM");
  }
}

if (process.argv[1] && realpathSync(process.argv[1]) === realpathSync(SCRIPT_PATH)) {
  capture(parseArgs(process.argv.slice(2))).catch((error) => {
    console.error(`management-baseline-072: ${error.stack ?? error}`);
    process.exitCode = 1;
  });
}
