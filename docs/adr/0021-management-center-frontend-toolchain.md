# ADR-0021: Management Center Frontend Toolchain

## Status

Accepted for 0.72.0 by the Management Center and release-governance maintainers.

## Context

Management Center 2.0 needs a richer operational interface without weakening the
existing single-binary deployment, same-origin security boundary, deterministic
packaging, or offline administration. The previous hand-maintained JavaScript bundle
had no static type boundary and duplicated source files into the server tree manually.

## Decision

Use strict TypeScript, Preact, and Vite. Application code is split into typed API,
state, router, component, history, and controller modules. Preact is the only runtime
dependency and is bundled into content-hashed production assets; no CDN or dynamic
runtime dependency is permitted.

The locked build produces `console/dist/manifest.json`. `embed-dist.mjs` validates the
manifest and asset policy, copies the exact bytes into the server crate, and generates
the Rust include table. Static and package tests reject source maps, external runtime
URLs, missing hashes, byte drift, path escape, mixed artifacts, and non-reproducible
build output.

The imperative controller remains temporarily responsible for polling and filling the
pre-rendered component shell. This preserves the already-tested truth-state and
read-only behaviour while establishing typed component and state boundaries for later
incremental replacement. It is not permission to introduce untyped DOM or write paths.

## Consequences

- `npm ci` from the committed lockfile is mandatory for candidate builds.
- Type checking, lint, unit coverage, deterministic-build, supply-chain, browser, and
  Rust embedding tests are release gates.
- The server serves only assets in the generated manifest table and returns 404 for
  unknown paths.
- Compatibility aliases are generated for artifact-mixing canaries; they are not
  authored application sources.
- Any dependency or build-tool change requires an updated lockfile, SBOM review, and
  deterministic candidate evidence.

## Revisit When

Revisit if the component runtime cannot meet the bundle, accessibility, security, or
deterministic-build budgets, or if the server gains a separately deployed console with
a different trust boundary.
