use std::collections::BTreeSet;

use hydracache::cluster_grid_metric_descriptors;

use crate::{HydraCacheOverview, HydraCacheRegistry};

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
        write_header(
            &mut out,
            "hydracache_cache_hits_total",
            "counter",
            "Cache hits.",
        );
        write_header(
            &mut out,
            "hydracache_cache_misses_total",
            "counter",
            "Cache misses.",
        );
        write_header(
            &mut out,
            "hydracache_cache_loads_total",
            "counter",
            "Loader executions.",
        );
        write_header(
            &mut out,
            "hydracache_cache_hit_ratio",
            "gauge",
            "Cache hit ratio.",
        );
        write_header(
            &mut out,
            "hydracache_cache_estimated_entries",
            "gauge",
            "Estimated local entries.",
        );

        for cache in &overview.caches {
            let cache_label = escape_label(&cache.name);
            push_metric(
                &mut out,
                "hydracache_cache_hits_total",
                &cache_label,
                cache.stats.hits,
            );
            push_metric(
                &mut out,
                "hydracache_cache_misses_total",
                &cache_label,
                cache.stats.misses,
            );
            push_metric(
                &mut out,
                "hydracache_cache_loads_total",
                &cache_label,
                cache.stats.loads,
            );
            push_metric_f64(
                &mut out,
                "hydracache_cache_hit_ratio",
                &cache_label,
                cache.stats.hit_ratio.unwrap_or(0.0),
            );
            push_metric(
                &mut out,
                "hydracache_cache_estimated_entries",
                &cache_label,
                cache.estimated_entries,
            );
        }
        out
    }
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
    ]);
    names.extend(
        cluster_grid_metric_descriptors()
            .iter()
            .map(|descriptor| descriptor.name),
    );
    names
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

fn push_metric(out: &mut String, name: &str, cache_label: &str, value: u64) {
    out.push_str(name);
    out.push_str("{cache=\"");
    out.push_str(cache_label);
    out.push_str("\"} ");
    out.push_str(&value.to_string());
    out.push('\n');
}

fn push_metric_f64(out: &mut String, name: &str, cache_label: &str, value: f64) {
    out.push_str(name);
    out.push_str("{cache=\"");
    out.push_str(cache_label);
    out.push_str("\"} ");
    out.push_str(&format!("{value:.6}"));
    out.push('\n');
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
