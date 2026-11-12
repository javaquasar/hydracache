package io.hydracache.imap.semantic;

import java.util.Map;
import java.util.Objects;
import java.util.Set;

/** Local-harness capabilities; this object cannot enable production dispatch. */
public record IMapCapabilityDescriptor(
    Set<String> plannedOperations, Set<String> operations,
    Map<String, Integer> bounds, Set<String> divergences,
    Set<String> unsupportedOperations, String scope, Set<String> interfaceFeatures) {
  public IMapCapabilityDescriptor(Set<String> plannedOperations, Set<String> operations,
      Map<String, Integer> bounds, Set<String> divergences, Set<String> unsupportedOperations) {
    this(plannedOperations, operations, bounds, divergences, unsupportedOperations, "bounded_reference", Set.of());
  }
  public IMapCapabilityDescriptor {
    plannedOperations = Set.copyOf(plannedOperations);
    operations = Set.copyOf(operations);
    bounds = Map.copyOf(bounds);
    divergences = Set.copyOf(divergences);
    unsupportedOperations = Set.copyOf(unsupportedOperations);
    interfaceFeatures = Set.copyOf(interfaceFeatures);
    if (!Set.of("bounded_reference", "unified_reference", "sdk_existing_narrow").contains(scope)) {
      throw new IllegalArgumentException("unknown capability scope");
    }
    if (operations.isEmpty() || !plannedOperations.containsAll(operations)
        || bounds.values().stream().anyMatch(bound -> bound <= 0)
        || operations.stream().anyMatch(unsupportedOperations::contains)) {
      throw new IllegalArgumentException("invalid capability descriptor");
    }
  }

  public static IMapCapabilityDescriptor unified(BoundedIMapApi.Bounds bounds) {
    var local = local(bounds);
    return new IMapCapabilityDescriptor(local.plannedOperations(), local.plannedOperations(), local.bounds(),
        local.divergences(), local.unsupportedOperations(), "unified_reference", lifecycleFeatures());
  }

  /** Existing SDK tokens only; productionEnabled concerns the expanded 0.75 capability. */
  public static IMapCapabilityDescriptor sdkActual() {
    Set<String> operations = Set.of("IMap.get", "IMap.put", "IMap.replace", "IMap.remove(key,value)",
        "IMap.addEntryListener", "IMap.getLock", "FencedLock.tryLock", "FencedLock.unlock",
        "FencedLock.renew", "FencedLock.isLocked", "FencedLock.getFence");
    return new IMapCapabilityDescriptor(operations, operations, Map.of(), Set.of("existing_sdk_surface_only", "no_expanded_075_claim"),
        Set.of("IMap.executeOnKey", "IMap.addInterceptor", "HazelcastInstance.getExecutorService", "HazelcastInstance.getSql",
            "ReplicatedMap", "Ringbuffer", "ReliableTopic", "PNCounter", "CPSubsystem.semaphore", "CPSubsystem.atomicLong"),
        "sdk_existing_narrow", Set.of());
  }

  public static Set<String> lifecycleFeatures() {
    return Set.of("safe_outcome_retirement", "aggregate_payload_budgets", "atomic_listener_resynchronization",
        "version_conditional_bulk_api", "bounded_batch_outcome_query", "incremental_expiry_cleanup", "runtime_service_checkpoint_restore", "bounded_batch_entry_views");
  }

  public static IMapCapabilityDescriptor local(BoundedIMapApi.Bounds bounds) {
    Objects.requireNonNull(bounds, "bounds");
    Set<String> planned = Set.of("get", "put", "delete", "conditional_replace",
        "conditional_remove", "contains_key", "put_if_absent", "replace_if_present",
        "get_and_remove", "get_and_put", "get_all", "put_all_detailed", "remove_all_detailed",
        "set_ttl", "remaining_ttl", "entry_listener", "size", "is_empty", "contains_value_bounded",
        "clear_detailed", "destroy", "evict", "evict_all_detailed", "key_set_page", "values_page",
        "entry_set_page", "get_or_default", "contains_entry", "set",
        "get_entry_view", "try_put", "try_remove", "replace_all_detailed", "remove_entry_listener",
        "capability_descriptor", "replace_if_version", "remove_if_version", "set_ttl_if_version",
        "query_operation_outcome", "local_statistics");
    return new IMapCapabilityDescriptor(planned,
        Set.of("put", "entry_listener", "size", "is_empty", "contains_value_bounded",
            "clear_detailed", "destroy", "evict", "evict_all_detailed", "key_set_page",
            "values_page", "entry_set_page", "get_or_default", "contains_entry", "set",
            "get_entry_view", "try_put", "try_remove", "replace_all_detailed",
            "remove_entry_listener", "capability_descriptor"),
        Map.of("key_bytes", bounds.maxKeyBytes(), "value_bytes", bounds.maxValueBytes(),
            "dedup_entries_per_tenant", bounds.maxEntries(), "scan_page_items", bounds.maxPageItems(),
            "scan_entries_per_request", bounds.maxScanItems(), "response_bytes", bounds.maxResponseBytes(),
            "bulk_mutation_entries", bounds.maxBulkMutationItems(),
            "request_entries", Math.min(bounds.maxBulkMutationItems(), BoundedIMapApi.MAX_REQUEST_ENTRIES),
            "request_bytes", BoundedIMapApi.MAX_REQUEST_BYTES),
        Set.of("local_reference_only", "bounded_iteration", "explicit_key_replacements",
            "no_distributed_transaction", "cancellation_after_admission_is_ambiguous"),
        Set.of("entry_processor", "compute", "merge", "map_store", "near_cache"));
  }

  public boolean productionEnabled() { return false; }

  public Set<String> unsupportedFamilies() {
    return Set.of("remote_entry_processor", "remote_predicate", "remote_interceptor",
        "distributed_transaction", "unbounded_scan");
  }

  public boolean supports(String operation) {
    return operations.contains(Objects.requireNonNull(operation, "operation"));
  }

  public void requireSupported(String operation) {
    if (!supports(operation)) throw new UnsupportedOperationException(operation);
  }
}
