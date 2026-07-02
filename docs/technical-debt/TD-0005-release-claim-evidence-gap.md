# TD-0005: Release-claim evidence gap (Hibernate L2 / JVM artifact)

## Status

Open (wording branch applied; artifact branch outstanding).

Owner: ecosystem / Java-migration roadmap.

Candidate target: the release that ships the actual `hydracache-hibernate` Java
artifact (and the wider Java toolkit) with a conformance gate. The overclaim has
been corrected in wording (see "Progress" below); the remaining debt is the
**absent artifact**, now honestly labelled "planned" rather than "shipped".

## Progress (2026-07 — wording branch)

Present-tense overclaims corrected to "Rust-side contract; Java artifact planned":
`docs/integrations/hibernate.md`, `docs/plans/INDEX.md` (0.49 row),
`docs/plans/releases.toml` (0.49 theme), `docs/POSITIONING.md`,
`docs/releases/0.49.0.md`, `docs/COMPAT.md`, `docs/adr/0006-…`,
`docs/integrations/java-migration.md`. Historical planning docs
(`docs/plans/V0_45..49`, `docs/tmp/…`) are left as-is (they are plans/design, not
current-state claims).

## Context

The `0.49` ecosystem release is listed in `docs/plans/INDEX.md` and
`docs/plans/releases.toml` as shipping a **"Hibernate L2 provider"**, and
`docs/integrations/hibernate.md` states, in the present tense:

> "The Java artifact is `hydracache-hibernate` and implements Hibernate's
> `RegionFactory` / `DomainDataRegion` SPI outside the Cargo workspace."

What actually exists in the repository:

- `crates/hydracache-client-protocol/src/hibernate.rs` — the **Rust-side** provider
  *contract* (`RegionMapping` builds the `Get`/`Put`/`Invalidate`/`EvictRegion`
  requests "the Java provider must send over protocol v1");
- `crates/hydracache-client-protocol/tests/hibernate_contract.rs` — tests the
  **Rust** request-shaping, not a live Hibernate `RegionFactory`;
- `docs/integrations/hibernate.md` + `docs/integrations/java-migration.md` — docs.

There is **no Java/JVM artifact** in the repo: a workspace scan finds **0 `.java`
files** and no `pom.xml`/`build.gradle`. The `hydracache-hibernate` artifact the
docs refer to does not exist here, is not built, and is not published.

The gap is **broader than Hibernate.** `docs/integrations/java-migration.md`
lists a whole Java toolkit — `hydracache-java-client`, the Spring Boot starters,
`hydracache-spring-cache`, `hydracache-jcache`, `hydracache-hibernate` — none of
which is in this workspace. That doc already carries an honest caveat that "no
buildable Maven/Gradle Java artifact is published from this workspace" (added for
`0.52`); this TD extends that honesty consistently to the `0.49` Hibernate claim
and the artifact list. Only the **Rust-side** protocol/provider contracts and
their conformance tests ship today.

## Why It Is A Debt

The wording claims a *shipped* JVM artifact ("The Java artifact **is**
`hydracache-hibernate` and **implements** …") while only the protocol contract
that such an artifact *would* consume is present. This is a present-tense
overclaim: a Hibernate/JVM team reading the release cannot depend on, build, or
`mvn` the provider, and the claim is **asserted, not demonstrated** — contrary to
the project's status-honesty discipline (RULES: "shipped = gates passed"; R-11
status honesty). The Rust contract + conformance-shaping is real and valuable;
the gap is only the *evidence for the Java artifact half of the claim*.

## Risk While Open

- A migrating Java team may assume a ready-to-use L2 provider exists.
- The "provider" claim cannot be tied to a CI conformance run against real
  Hibernate.
- Positioning/roadmap credibility erodes if one claim is found to overreach.

## Revisit Triggers

Resolve (either branch closes this debt) when one of:

- **Wording branch:** the claim is corrected everywhere it appears
  (`INDEX.md`, `releases.toml` `0.49` theme, `docs/releases/0.49.0.md`,
  `docs/integrations/hibernate.md`, `POSITIONING.md`) to describe a **Rust-side
  provider *contract*** with the **Java artifact as planned/future**, not shipped;
  or
- **Artifact branch:** a real `hydracache-hibernate` Java/Maven module is added
  (even outside the Cargo workspace) implementing `RegionFactory` /
  `DomainDataRegion`, with a conformance suite that exercises the protocol-v1
  contract and runs in a named CI gate.

## How To Verify The Debt Can Be Removed Safely

- Wording branch: grep the repo for "Hibernate L2 provider" / "The Java artifact
  is" and confirm every occurrence states contract-only + planned Java artifact;
  no present-tense "implements" claim remains without a corresponding artifact.
- Artifact branch: the Java module builds, a `RegionFactory` conformance test runs
  green against protocol v1 in CI, and the protocol shape is COMPAT-registered
  (R-4); only then may the wording say "provider" in the present tense.

## Related Plans

- `docs/plans/V0_49_ECOSYSTEM_AND_EXTERNAL_CONSUMERS_PLAN.md`
- `docs/plans/V0_49_SCOPE_AND_HARDENING_PATCH.md`
- `docs/integrations/hibernate.md`, `docs/integrations/java-migration.md`
