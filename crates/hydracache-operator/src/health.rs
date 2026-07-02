//! Health/readiness invariants for operator-owned Kubernetes resources.

use k8s_openapi::api::apps::v1::StatefulSet;
use k8s_openapi::api::core::v1::Service;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

use crate::resources::{ADMIN_PORT, SERVER_CONTAINER};

pub const HEALTHZ_PATH: &str = "/healthz";
pub const READYZ_PATH: &str = "/readyz";

/// Kubernetes Services route only Ready endpoints unless publishNotReadyAddresses is set.
pub fn client_service_routes_only_ready(service: &Service) -> bool {
    !service
        .spec
        .as_ref()
        .and_then(|spec| spec.publish_not_ready_addresses)
        .unwrap_or(false)
}

/// Return whether the server container readiness probe gates admission on `/readyz`.
pub fn statefulset_uses_admission_readiness_gate(stateful_set: &StatefulSet) -> bool {
    let Some(container) = server_container(stateful_set) else {
        return false;
    };
    container
        .readiness_probe
        .as_ref()
        .and_then(|probe| probe.http_get.as_ref())
        .is_some_and(|http| {
            http.path.as_deref() == Some(READYZ_PATH)
                && http.port == IntOrString::String("admin".to_owned())
        })
}

/// Return whether liveness is wired to the W0 admin health surface.
pub fn statefulset_uses_admin_liveness_probe(stateful_set: &StatefulSet) -> bool {
    let Some(container) = server_container(stateful_set) else {
        return false;
    };
    container
        .liveness_probe
        .as_ref()
        .and_then(|probe| probe.http_get.as_ref())
        .is_some_and(|http| {
            http.path.as_deref() == Some(HEALTHZ_PATH)
                && http.port == IntOrString::String("admin".to_owned())
        })
}

/// Return whether the admin port is exposed for probes and operator actions.
pub fn statefulset_exposes_admin_port(stateful_set: &StatefulSet) -> bool {
    server_container(stateful_set).is_some_and(|container| {
        container.ports.as_ref().is_some_and(|ports| {
            ports.iter().any(|port| {
                port.name.as_deref() == Some("admin") && port.container_port == ADMIN_PORT
            })
        })
    })
}

fn server_container(stateful_set: &StatefulSet) -> Option<&k8s_openapi::api::core::v1::Container> {
    stateful_set
        .spec
        .as_ref()
        .and_then(|spec| spec.template.spec.as_ref())
        .and_then(|spec| {
            spec.containers
                .iter()
                .find(|container| container.name == SERVER_CONTAINER)
        })
}
