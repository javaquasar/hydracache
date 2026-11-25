# kube-client 4.0.0 TLS backport

The Kubernetes operator uses kube-rs 4.0.0 to support Rust 1.88. The
[RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2026-0600) covers
OpenSSL CA handling. The first fixed upstream release, 4.2.1, requires Rust
1.89. This copy applies the [upstream fix and regression tests](https://github.com/kube-rs/kube/commit/bc27fdc58380d8ba78347145999193e00fe1b030)
to 4.0.0.

The base is the crates.io `kube-client-4.0.0.crate` archive, SHA-256
`d0f628e05bc2264c21fe10d3d675117dc9b43ea3bf4fb07262a222679757537b`.
The archive records source commit `2a14decb8a9e879d27239af09329a87bd10f045f`.
The Rust changes are confined to `src/client/tls.rs`. The Cargo manifests
select this copy for the private operator and its standalone tests.
The operator retains its Rustls features.

The standalone test lockfile pins `hyper-openssl` 0.10.2 because 0.10.3 pulls
in dependencies that require Rust 1.95. Run the upstream OpenSSL tests on a
host with OpenSSL development headers.

```sh
cargo +1.88.0 test --manifest-path vendor/kube-client/Cargo.toml --locked \
    --lib --no-default-features --features client,openssl-tls client::tls::openssl_tls::tests
```

Check the operator with `cargo +1.88.0 check -p hydracache-operator
--all-targets --locked` and run `cargo deny check` from the workspace root.
Review new kube-client advisories against this copy while the override is
in place. Drop the override when an upstream release containing the fix
supports the workspace MSRV.
