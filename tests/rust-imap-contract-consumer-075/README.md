# External development contract consumer

`cargo xtask imap-pre074-tools --release 0.75` copies this source to a fresh, standalone workspace
under `target/imap-075/`, seeds its dependency resolution from the repository lockfile, builds it
and runs its tests. It cannot rely on workspace-only imports or unpublished additions to the SDK.
The dependency is the explicitly non-published cluster testkit: this checks the development
backend boundary, not the packaged production Rust SDK. Release SDK qualification stays blocked.
