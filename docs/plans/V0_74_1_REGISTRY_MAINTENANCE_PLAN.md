# HydraCache 0.74.1 registry maintenance

> **At a glance**
>
> - **What** — publish the 0.74 library set from registry-resolvable dependencies.
> - **Why** — a consumer cannot inherit the release workspace's dependency patches.
> - **After (depends on)** — 0.74.0.
> - **Unblocks** — 0.74 maintenance packages on crates.io.
> - **Status:** shipped for registry maintenance. The tagged CI and public
>   consumer checks are enforced by the publication workflow.

The Rust minimum remains 1.88. The migration preserves the 0.74 maintenance
wire and disk formats. Performance qualification remains subject to the
existing 0.74 requirements.

## W0. Fork source and package validation

Bind each fork to its upstream release archive and reviewed source. Check
its normalized manifest, native SQLite resolution and RustSec policy.
Complete the SQLx standalone packaging gate before admitting the facade.

## W1. Registry consumer and compatibility

Resolve and lock the published fork names in HydraCache. Check the workspace
and packaged SQLx/Diesel, Raft, durable-store and daemon consumers on Rust
1.88 without consumer patches or RustSec exceptions. Check upgrade and
rollback on the resulting artifacts before making a new binary compatibility
claim.

## W2. Release admission

Verify the publishable package list and order, exact tagged CI and release
notes. Run post-publication consumer checks on stable and Rust 1.88. The
owner authorizes any registry or GitHub release operation.

The current dependency notes and packaging boundary are in the
[0.74.1 release notes](../releases/0.74.1.md).
