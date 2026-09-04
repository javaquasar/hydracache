# W11 security, accessibility, and supply-chain evidence

Scope: the 0.72 Management Center additions. The exact release commit is bound later by W14; this
file records reproducible local gate definitions and known admission state, not a ship receipt.

## Green gates

- `cargo test -p hydracache-server --test management_security_072 --locked`: four integration
  tests cover the 17-route authorization matrix, security headers, XSS-safe pseudonymization, and
  independent management-read saturation/admin fairness.
- `cargo test -p hydracache-server --lib --locked management_security`: direct permit saturation
  and drop/cancellation cleanup.
- `cargo test -p hydracache-server --locked`: complete server suite green; Docker/kind and manual
  soak tests remain explicitly ignored according to their existing gates.
- `cargo clippy -p hydracache-server --all-targets --locked -- -D warnings` and package rustfmt:
  green.
- `npm --prefix console test`: 8 unit plus 32 Playwright project executions green across desktop
  and narrow mobile, including axe, keyboard, forced colors, reduced motion, hostile text,
  GET-only networking, responsive and effective tablet/200%-zoom checks.
- `npm --prefix console run build`: embedded asset drift, raw-HTML/eval/write markers, external
  runtime dependencies, accessibility markers and credential-pattern scan green.
- `npm --prefix console run supply-chain`: exact direct pins, npm-registry provenance, integrity,
  reviewed MIT/Apache-2.0/MPL-2.0 licenses green; deterministic CycloneDX 1.5 SBOM emitted.
- `npm --prefix console audit --audit-level=high`: zero vulnerabilities.
- `cargo xtask doc-check`: release documentation registry green.

## Falsifiability

With `HYDRACACHE_CANARY_DEFECT=MC72-W11-XSS-AUTH-BYPASS`, the registered test fails with
`HC-CANARY-RED:MC72-W11-XSS-AUTH-BYPASS`. With the candidate restored, that same test is green.

## Resolved workspace-wide blocker

The original W11 run was intentionally retained as red: unapproved MIT-0/NCSA expressions, the
unlicensed local fuzz package, and yanked `chacha20 0.10.1`/`spin 0.9.8`. W13 resolved rather than
waived it: `h2` is 0.4.19, `chacha20` is 0.10.2, `spin` is 0.9.9, OSI-approved MIT-0/NCSA are
explicitly reviewed, and `hydracache-fuzz` inherits the workspace Apache-2.0 license. The repeated
full `cargo deny check` reports advisories, bans, licenses and sources all green.
