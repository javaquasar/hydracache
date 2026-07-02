use std::collections::BTreeSet;

use hydracache::{cluster_grid_metric_descriptors, ClusterGridCounters};

use crate::{ClusterTopologyOverview, HydraCacheOverview, HydraCacheRegistry};

/// Framework-neutral Prometheus text exporter.
#[derive(Debug, Clone)]
pub struct PrometheusExporter {
    registry: HydraCacheRegistry,
}

impl PrometheusExporter {
    /// Create an exporter from a cache registry.
    pub fn new(registry: HydraCacheRegistry) -> Self {
        Self { registry }
    }

    /// Render the current registry snapshot as Prometheus text.
    pub async fn render(&self) -> String {
        Self::render_overview(&self.registry.overview().await)
    }

    /// Render an already captured overview as Prometheus text.
    pub fn render_overview(overview: &HydraCacheOverview) -> String {
        let mut out = String::new();
        write_cache_metrics(&mut out, overview);
        write_admission_metrics(&mut out, overview);
        write_cluster_grid_metrics(&mut out, &overview.cluster_grid);
        write_topology_metrics(&mut out, &overview.topology, overview.backup_age_seconds);
        out
    }
}

fn write_cache_metrics(out: &mut String, overview: &HydraCacheOverview) {
    write_header(out, "hydracache_cache_hits_total", "counter", "Cache hits.");
    write_header(
        out,
        "hydracache_cache_misses_total",
        "counter",
        "Cache misses.",
    );
    write_header(
        out,
        "hydracache_cache_loads_total",
        "counter",
        "Loader executions.",
    );
    write_header(
        out,
        "hydracache_cache_hit_ratio",
        "gauge",
        "Cache hit ratio.",
    );
    write_header(
        out,
        "hydracache_cache_estimated_entries",
        "gauge",
        "Estimated local entries.",
    );

    for cache in &overview.caches {
        push_metric_labeled(
            out,
            "hydracache_cache_hits_total",
            &[("cache", cache.name.as_str())],
            cache.stats.hits,
        );
        push_metric_labeled(
            out,
            "hydracache_cache_misses_total",
            &[("cache", cache.name.as_str())],
            cache.stats.misses,
        );
        push_metric_labeled(
            out,
            "hydracache_cache_loads_total",
            &[("cache", cache.name.as_str())],
            cache.stats.loads,
        );
        push_metric_f64_labeled(
            out,
            "hydracache_cache_hit_ratio",
            &[("cache", cache.name.as_str())],
            cache.stats.hit_ratio.unwrap_or(0.0),
        );
        push_metric_labeled(
            out,
            "hydracache_cache_estimated_entries",
            &[("cache", cache.name.as_str())],
            cache.estimated_entries,
        );
    }
}

fn write_admission_metrics(out: &mut String, overview: &HydraCacheOverview) {
    write_header(
        out,
        "hydracache_admission_rejected_total",
        "counter",
        "Admission requests rejected by overload control.",
    );
    write_header(
        out,
        "hydracache_admission_in_flight",
        "gauge",
        "Current admitted operation count.",
    );
    write_header(
        out,
        "hydracache_admission_queue_depth",
        "gauge",
        "Current admission queue depth.",
    );
    push_metric_plain(
        out,
        "hydracache_admission_rejected_total",
        overview.admission.rejected_total,
    );
    push_metric_plain(
        out,
        "hydracache_admission_in_flight",
        overview.admission.in_flight,
    );
    push_metric_plain(
        out,
        "hydracache_admission_queue_depth",
        overview.admission.queue_depth,
    );
}

fn write_cluster_grid_metrics(out: &mut String, counters: &ClusterGridCounters) {
    for descriptor in cluster_grid_metric_descriptors() {
        write_header(
            out,
            descriptor.name,
            metric_type(descriptor.name),
            "HydraCache cluster-grid metric.",
        );
        push_grid_metric(
            out,
            descriptor.name,
            descriptor.labels,
            cluster_counter_value(counters, descriptor.name),
        );
    }
}

fn write_topology_metrics(
    out: &mut String,
    topology: &ClusterTopologyOverview,
    backup_age_seconds: Option<u64>,
) {
    write_header(
        out,
        "hydracache_cluster_members",
        "gauge",
        "Visible cluster member count.",
    );
    write_header(
        out,
        "hydracache_cluster_leader",
        "gauge",
        "Cluster leader indicator.",
    );
    write_header(
        out,
        "hydracache_cluster_epoch",
        "gauge",
        "Current cluster authority epoch.",
    );
    write_header(
        out,
        "hydracache_cluster_reshard_phase",
        "gauge",
        "Current cluster reshard phase.",
    );
    write_header(
        out,
        "hydracache_backup_age_seconds",
        "gauge",
        "Worst known backup age in seconds.",
    );

    let source = topology.source.as_label();
    push_metric_labeled(
        out,
        "hydracache_cluster_members",
        &[("source", source)],
        topology.members,
    );
    push_metric_labeled(
        out,
        "hydracache_cluster_leader",
        &[
            ("source", source),
            ("node", topology.leader.as_deref().unwrap_or("none")),
        ],
        u64::from(topology.leader.is_some()),
    );
    push_metric_labeled(
        out,
        "hydracache_cluster_epoch",
        &[("source", source)],
        topology.epoch,
    );
    push_metric_labeled(
        out,
        "hydracache_cluster_reshard_phase",
        &[
            ("source", source),
            ("phase", topology.reshard_phase.as_label()),
        ],
        1,
    );
    push_metric_labeled(
        out,
        "hydracache_backup_age_seconds",
        &[("source", source)],
        backup_age_seconds.unwrap_or_default(),
    );
}

/// Return metric names exported or reserved by the production operator surface.
pub fn registered_metric_names() -> BTreeSet<&'static str> {
    let mut names = BTreeSet::from([
        "hydracache_cache_hits_total",
        "hydracache_cache_misses_total",
        "hydracache_cache_loads_total",
        "hydracache_cache_hit_ratio",
        "hydracache_cache_estimated_entries",
        "hydracache_admission_rejected_total",
        "hydracache_admission_in_flight",
        "hydracache_admission_queue_depth",
        "hydracache_cluster_members",
        "hydracache_cluster_leader",
        "hydracache_cluster_epoch",
        "hydracache_cluster_reshard_phase",
        "hydracache_backup_age_seconds",
    ]);
    names.extend(
        cluster_grid_metric_descriptors()
            .iter()
            .map(|descriptor| descriptor.name),
    );
    names
}

fn cluster_counter_value(counters: &ClusterGridCounters, name: &str) -> u64 {
    match name {
        "hydracache_replication_success_total" => counters.replication_success_total,
        "hydracache_replication_failure_total" => counters.replication_failure_total,
        "hydracache_bytes_replicated_total" => counters.bytes_replicated_total,
        "hydracache_replication_backpressure_total" => counters.replication_backpressure_total,
        "hydracache_replication_oversized_rejected_total" => {
            counters.replication_oversized_rejected_total
        }
        "hydracache_under_replicated_keys" => counters.under_replicated_keys,
        "hydracache_topology_fence_rejected_total" => counters.topology_fence_rejected_total,
        "hydracache_tombstone_repair_debt" => counters.tombstone_repair_debt,
        "hydracache_replicated_value_rejected_total" => counters.replicated_value_rejected_total,
        "hydracache_split_brain_detected_total" => counters.split_brain_detected_total,
        "hydracache_merge_discarded_entries_total" => counters.merge_discarded_entries_total,
        "hydracache_merge_unresolved_conflicts_total" => counters.merge_unresolved_conflicts_total,
        "hydracache_cluster_auth_rejected_total" => counters.cluster_auth_rejected_total,
        "hydracache_repair_debt_degraded_mode" => counters.repair_debt_degraded_mode,
        "hydracache_placement_zone_underspread" => counters.placement_zone_underspread,
        "hydracache_reshard_moves_inflight" => counters.reshard_moves_inflight,
        "hydracache_reshard_backfill_lag" => counters.reshard_backfill_lag,
        "hydracache_read_local_zone_total" => counters.read_local_zone_total,
        "hydracache_read_hedged_total" => counters.read_hedged_total,
        "hydracache_read_hedge_win_total" => counters.read_hedge_win_total,
        "hydracache_value_tier_promotions_total" => counters.value_tier_promotions_total,
        "hydracache_value_tier_demotions_total" => counters.value_tier_demotions_total,
        "hydracache_invalidate_batch_total" => counters.invalidate_batch_total,
        "hydracache_invalidation_saga_pending" => counters.invalidation_saga_pending,
        "hydracache_auto_repair_active_total" => counters.auto_repair_active_total,
        "hydracache_auto_repair_advisory_total" => counters.auto_repair_advisory_total,
        "hydracache_op_consistency_level_total" => counters.consistency_level_operations_total,
        "hydracache_consistency_unsatisfiable_total" => counters.consistency_unsatisfiable_total,
        "hydracache_hints_stored_total" => counters.hints_stored_total,
        "hydracache_hints_replayed_total" => counters.hints_replayed_total,
        "hydracache_hints_dropped_total" => counters.hints_dropped_total,
        "hydracache_hint_store_bytes" => counters.hint_store_bytes,
        "hydracache_repair_ranges_exchanged_total" => counters.repair_ranges_exchanged_total,
        "hydracache_read_repair_total" => counters.read_repair_total,
        "hydracache_repair_progress_ratio" => counters.repair_progress_ratio,
        "hydracache_peer_phi" => counters.peer_phi_scaled,
        "hydracache_false_suspect_total" => counters.false_suspect_total,
        "hydracache_cas_applied_total" => counters.cas_applied_total,
        "hydracache_cas_mismatch_total" => counters.cas_mismatch_total,
        "hydracache_lock_acquired_total" => counters.lock_acquired_total,
        "hydracache_lock_stale_token_rejected_total" => counters.lock_stale_token_rejected_total,
        "hydracache_invalidation_ring_depth" => counters.invalidation_ring_depth,
        "hydracache_invalidation_replayed_total" => counters.invalidation_replayed_total,
        "hydracache_invalidation_fell_behind_total" => counters.invalidation_fell_behind_total,
        "hydracache_invalidation_ring_overrun_total" => counters.invalidation_ring_overrun_total,
        "hydracache_session_watermark_entries" => counters.session_watermark_entries,
        "hydracache_session_active_sessions" => counters.session_active_sessions,
        "hydracache_session_watermark_entries_p99" => counters.session_watermark_entries_p99,
        "hydracache_session_worst_staleness_versions" => counters.session_worst_staleness_versions,
        "hydracache_session_watermark_coarsened_total" => {
            counters.session_watermark_coarsened_total
        }
        "hydracache_session_token_rejected_total" => counters.session_token_rejected_total,
        "hydracache_session_ryw_escalations_total" => counters.session_ryw_escalations_total,
        "hydracache_session_guarantee_unmet_total" => counters.session_guarantee_unmet_total,
        "hydracache_monotonic_read_violations_prevented_total" => {
            counters.monotonic_read_violations_prevented_total
        }
        "hydracache_monotonic_write_reorders_prevented_total" => {
            counters.monotonic_write_reorders_prevented_total
        }
        "hydracache_causal_writes_deferred_total" => counters.causal_writes_deferred_total,
        "hydracache_causal_summary_coarsened_total" => counters.causal_summary_coarsened_total,
        "hydracache_causal_dependency_bytes" => counters.causal_dependency_bytes,
        "hydracache_bounded_staleness_fast_serves_total" => {
            counters.bounded_staleness_fast_serves_total
        }
        "hydracache_bounded_staleness_escalations_total" => {
            counters.bounded_staleness_escalations_total
        }
        _ => 0,
    }
}

fn metric_type(name: &str) -> &'static str {
    if name.ends_with("_total") {
        "counter"
    } else {
        "gauge"
    }
}

fn write_header(out: &mut String, name: &str, metric_type: &str, help: &str) {
    out.push_str("# HELP ");
    out.push_str(name);
    out.push(' ');
    out.push_str(help);
    out.push('\n');
    out.push_str("# TYPE ");
    out.push_str(name);
    out.push(' ');
    out.push_str(metric_type);
    out.push('\n');
}

fn push_metric_plain(out: &mut String, name: &str, value: u64) {
    out.push_str(name);
    out.push(' ');
    out.push_str(&value.to_string());
    out.push('\n');
}

fn push_grid_metric(out: &mut String, name: &str, labels: &[&str], value: u64) {
    let aggregate_labels = labels
        .iter()
        .map(|&label| (label, aggregate_label_value(label)))
        .collect::<Vec<_>>();
    push_metric_labeled(out, name, &aggregate_labels, value);
}

fn aggregate_label_value(label: &str) -> &'static str {
    match label {
        "state" => "unknown",
        _ => "aggregate",
    }
}

fn push_metric_labeled(out: &mut String, name: &str, labels: &[(&str, &str)], value: u64) {
    out.push_str(name);
    push_labels(out, labels);
    out.push(' ');
    out.push_str(&value.to_string());
    out.push('\n');
}

fn push_metric_f64_labeled(out: &mut String, name: &str, labels: &[(&str, &str)], value: f64) {
    out.push_str(name);
    push_labels(out, labels);
    out.push(' ');
    out.push_str(&format!("{value:.6}"));
    out.push('\n');
}

fn push_labels(out: &mut String, labels: &[(&str, &str)]) {
    if labels.is_empty() {
        return;
    }
    out.push('{');
    for (index, (name, value)) in labels.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str(name);
        out.push_str("=\"");
        out.push_str(&escape_label(value));
        out.push('"');
    }
    out.push('}');
}

fn escape_label(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}
