//! Local capability introspection derived from the checked-in provisional contracts.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde_json::Value;

pub const BOUNDED_OPERATIONS: &[&str] = &[
    "put",
    "entry_listener",
    "size",
    "is_empty",
    "contains_value_bounded",
    "clear_detailed",
    "destroy",
    "evict",
    "evict_all_detailed",
    "key_set_page",
    "values_page",
    "entry_set_page",
    "get_or_default",
    "contains_entry",
    "set",
    "get_entry_view",
    "try_put",
    "try_remove",
    "replace_all_detailed",
    "remove_entry_listener",
    "capability_descriptor",
];
pub const UNIFIED_OPERATIONS: &[&str] = &[
    "get",
    "put",
    "delete",
    "conditional_replace",
    "conditional_remove",
    "contains_key",
    "put_if_absent",
    "replace_if_present",
    "get_and_remove",
    "get_and_put",
    "get_all",
    "put_all_detailed",
    "remove_all_detailed",
    "set_ttl",
    "remaining_ttl",
    "entry_listener",
    "size",
    "is_empty",
    "contains_value_bounded",
    "clear_detailed",
    "destroy",
    "evict",
    "evict_all_detailed",
    "key_set_page",
    "values_page",
    "entry_set_page",
    "get_or_default",
    "contains_entry",
    "set",
    "get_entry_view",
    "try_put",
    "try_remove",
    "replace_all_detailed",
    "remove_entry_listener",
    "capability_descriptor",
    "replace_if_version",
    "remove_if_version",
    "set_ttl_if_version",
    "query_operation_outcome",
    "local_statistics",
];
pub const LIFECYCLE_FEATURES: &[&str] = &[
    "safe_outcome_retirement",
    "aggregate_payload_budgets",
    "atomic_listener_resynchronization",
    "version_conditional_bulk_api",
    "bounded_batch_outcome_query",
    "incremental_expiry_cleanup",
    "runtime_service_checkpoint_restore",
    "bounded_batch_entry_views",
];
pub const SDK_OPERATIONS: &[&str] = &[
    "IMap.get",
    "IMap.put",
    "IMap.replace",
    "IMap.remove(key,value)",
    "IMap.addEntryListener",
    "IMap.getLock",
    "FencedLock.tryLock",
    "FencedLock.unlock",
    "FencedLock.renew",
    "FencedLock.isLocked",
    "FencedLock.getFence",
];

/// Actual checked-in reference or legacy SDK families, distinct from planned surface projections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalCapabilityDescriptor {
    pub scope: &'static str,
    /// The expanded 0.75 production capability; this does not disable existing legacy SDK APIs.
    pub production_enabled: bool,
    pub operations: BTreeSet<String>,
    pub interface_features: BTreeSet<String>,
}

impl LocalCapabilityDescriptor {
    pub fn bounded() -> Self {
        Self::new("bounded_reference", BOUNDED_OPERATIONS, &[])
    }
    pub fn unified() -> Self {
        Self::new("unified_reference", UNIFIED_OPERATIONS, LIFECYCLE_FEATURES)
    }
    pub fn sdk_actual() -> Self {
        Self::new("sdk_existing_narrow", SDK_OPERATIONS, &[])
    }
    fn new(scope: &'static str, operations: &[&str], features: &[&str]) -> Self {
        Self {
            scope,
            production_enabled: false,
            operations: operations.iter().map(|id| (*id).into()).collect(),
            interface_features: features.iter().map(|id| (*id).into()).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityError(pub String);

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CapabilityError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiBound {
    pub minimum: u64,
    pub maximum: u64,
    pub unit: String,
    pub enforcement: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceSupport {
    Planned,
    Unsupported { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationCapability {
    pub id: String,
    pub disposition: String,
    pub outcomes: Vec<String>,
    pub errors: Vec<String>,
    pub ttl_directives: Vec<String>,
    pub retry_class: String,
    pub bounds: BTreeMap<String, ApiBound>,
    pub support: SurfaceSupport,
}

/// Surface support is a planned semantic projection, never runtime production readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub surface: String,
    pub release: String,
    pub production_enabled: bool,
    pub operations: BTreeMap<String, OperationCapability>,
    pub interface_features: BTreeSet<String>,
    pub unsupported_families: BTreeSet<String>,
}

impl CapabilityDescriptor {
    pub fn for_surface(surface: &str) -> Result<Self, CapabilityError> {
        Self::from_contracts(
            surface,
            include_str!("../../../docs/testing/imap/0.75/operation-contract.json"),
            include_str!("../../../docs/testing/imap/0.75/resource-bounds.json"),
            include_str!("../../../docs/testing/imap/0.75/surface-equivalence.json"),
            include_str!("../../../docs/testing/imap/0.75/status.json"),
        )
    }

    pub fn from_contracts(
        surface: &str,
        operations_json: &str,
        bounds_json: &str,
        surfaces_json: &str,
        status_json: &str,
    ) -> Result<Self, CapabilityError> {
        let operation_contract = contract(operations_json)?;
        let bounds_contract = contract(bounds_json)?;
        let surfaces_contract = contract(surfaces_json)?;
        let status = contract(status_json)?;
        if string(&status, "production_capability")? != "disabled_fail_closed" {
            return fail("provisional descriptor cannot enable production capability");
        }
        let extensions = &operation_contract["api_extensions"];
        if extensions["production_enabled"].as_bool() != Some(false) {
            return fail("api_extensions must explicitly disable production");
        }
        let mut bounds = BTreeMap::new();
        for bound in array(&bounds_contract, "bounds")? {
            let id = string(bound, "id")?.to_owned();
            let minimum = number(bound, "minimum")?;
            let maximum = number(bound, "maximum")?;
            if maximum == 0 || minimum > maximum {
                return fail("invalid capability bound");
            }
            let value = ApiBound {
                minimum,
                maximum,
                unit: string(bound, "unit")?.into(),
                enforcement: string(bound, "enforcement")?.into(),
            };
            if bounds.insert(id, value).is_some() {
                return fail("duplicate capability bound");
            }
        }
        let surfaces = array(&surfaces_contract, "surfaces")?;
        let matching = surfaces
            .iter()
            .filter(|item| item["id"].as_str() == Some(surface))
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return fail("unknown or duplicate capability surface");
        }
        let mut cells = BTreeMap::new();
        for cell in array(matching[0], "cells")? {
            let id = string(cell, "operation")?.to_owned();
            let support = match string(cell, "disposition")? {
                "enabled" if cell["unsupported_error"].is_null() => SurfaceSupport::Planned,
                "unsupported" => SurfaceSupport::Unsupported {
                    reason: string(cell, "unsupported_error")?.into(),
                },
                _ => return fail("invalid capability surface disposition"),
            };
            if cells.insert(id, support).is_some() {
                return fail("duplicate capability surface cell");
            }
        }
        let mut operations = BTreeMap::new();
        for operation in array(&operation_contract, "operations")? {
            let id = string(operation, "id")?.to_owned();
            let support = cells
                .remove(&id)
                .ok_or_else(|| CapabilityError(format!("missing surface cell for {id}")))?;
            let mut operation_bounds = BTreeMap::new();
            for name in strings(operation, "bound_ids")? {
                let bound = bounds
                    .get(&name)
                    .ok_or_else(|| CapabilityError(format!("unknown capability bound {name}")))?;
                operation_bounds.insert(name, bound.clone());
            }
            let disposition = string(operation, "disposition")?.to_owned();
            if !matches!(
                disposition.as_str(),
                "supported" | "supported_with_documented_divergence" | "unsupported"
            ) {
                return fail("invalid operation disposition");
            }
            let capability = OperationCapability {
                id: id.clone(),
                disposition,
                outcomes: strings(operation, "outcomes")?,
                errors: strings(operation, "errors")?,
                ttl_directives: strings(operation, "ttl_policy")?,
                retry_class: string(operation, "retry_class")?.into(),
                bounds: operation_bounds,
                support,
            };
            if operations.insert(id, capability).is_some() {
                return fail("duplicate capability operation");
            }
        }
        if !cells.is_empty() || operations.is_empty() {
            return fail("capability surface operation set differs from contract");
        }
        let mut interface_features = BTreeSet::new();
        for feature in array(extensions, "interface_features")? {
            if !interface_features.insert(string(feature, "id")?.into()) {
                return fail("duplicate interface feature");
            }
        }
        let unsupported = strings(extensions, "unsupported_families")?;
        let unsupported_families = unsupported.iter().cloned().collect::<BTreeSet<_>>();
        if unsupported_families.len() != unsupported.len() || unsupported_families.is_empty() {
            return fail("unsupported capability families must be unique and explicit");
        }
        Ok(Self {
            surface: surface.into(),
            release: "0.75".into(),
            production_enabled: false,
            operations,
            interface_features,
            unsupported_families,
        })
    }

    pub fn operation(&self, id: &str) -> Option<&OperationCapability> {
        self.operations.get(id)
    }
}

fn contract(source: &str) -> Result<Value, CapabilityError> {
    let value: Value =
        serde_json::from_str(source).map_err(|error| CapabilityError(error.to_string()))?;
    if value["release"] != "0.75"
        || value["contract_version"] != 1
        || value["status"] != "provisional"
    {
        return fail("unsupported capability contract identity");
    }
    Ok(value)
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, CapabilityError> {
    value[key]
        .as_str()
        .filter(|text| !text.is_empty())
        .ok_or_else(|| CapabilityError(format!("missing capability string {key}")))
}

fn number(value: &Value, key: &str) -> Result<u64, CapabilityError> {
    value[key]
        .as_u64()
        .ok_or_else(|| CapabilityError(format!("missing capability number {key}")))
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a [Value], CapabilityError> {
    value[key]
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| CapabilityError(format!("missing capability array {key}")))
}

fn strings(value: &Value, key: &str) -> Result<Vec<String>, CapabilityError> {
    array(value, key)?
        .iter()
        .map(|item| {
            item.as_str()
                .filter(|text| !text.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| CapabilityError(format!("invalid capability string list {key}")))
        })
        .collect()
}

fn fail<T>(message: &str) -> Result<T, CapabilityError> {
    Err(CapabilityError(message.into()))
}
