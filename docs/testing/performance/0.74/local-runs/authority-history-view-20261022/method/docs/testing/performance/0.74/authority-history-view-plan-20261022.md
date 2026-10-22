# Isolated historical authority reader view

Starting tree `33c54b0c`. Implement a sparse historical source/evidence view from
that exact Git commit, not a fallback to the current worktree. A read-only
discovery of the unchanged audit identifies 129 source files, 18 reader modules
and 24 sealed packets with 1423 members. Freeze those inputs before implementing
the view. Do not modify historical readers, evidence or runtime/source guards.

Materialize only allowlisted Git archive entries in a unique generated directory
under target. Verify source digests, all packet inventories/members, membership,
bounds, path safety, duplicate names and link absence before executing any reader.
Use a fresh isolated Python worker importing the unchanged readers only from that
view. Require exact equality with the archived audit facts and preserve the clean
model gate versus the older broad isolation refusal. Deny Path.open writes or
reads outside the view during analysis; this is a diagnostic guard for the pinned
reader code, not a security sandbox for arbitrary Python. Reverify the view after
analysis and clean up only its validated generated directory.

Keep a separate current-baseline identity guard. Historical success must never
admit changed current runtime. Demonstrate that distinction using an isolated
temporary candidate source copy: history remains replayable while changed current
source refuses. This identity guard is not the future owned-storage semantic
guard and does not authorize a runtime patch or migration of existing Rust guards.

Add unit/mutation controls for manifests, archive membership, traversal/links,
hashes/bounds, read-only scope, source drift and distinct decisions. Execute one
first source-identified replay and retain its complete receipt without replacing
old facts. Run existing offline/source checks, format/check/strict clippy and local
docs/registry/performance contracts. Seal the method and new evidence; update the
article/README/readiness plan with the remaining candidate-observer/guard boundary.

No model/measurement image, live profile, original-owner/fault attempt, WSL,
clock/service change, server/install/supervisor or product action. No allocation,
timing, layout, RSS or copy claim. Original 300/60/10 windows, first refusals,
qualification and all real publication/all-writer/full-fault/reconnect/server
admission gates stay unchanged.
