//! Kubernetes operator types and manifest helpers for HydraCache.

use std::sync::Once;

pub mod backup;
pub mod controller;
pub mod crd;
pub mod health;
pub mod persistence;
pub mod resources;
pub mod scale;
pub mod tls;
pub mod upgrade;

pub fn install_default_rustls_provider() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}
