#!/usr/bin/env bash
set -euo pipefail

cargo test -p hydracache --lib --locked -- --test-threads=1
cargo test -p hydracache --test allocation_profile --test lock_lease --test conditional_tombstone --locked -- --test-threads=1
cargo test -p hydracache-client-transport-axum --lib diagnostic_reset_reports_and_clears_every_mutable_data_owner --locked -- --test-threads=1
cargo test -p hydracache-client-hc2 --test reconnect_repair reset_reconnects_once_repairs_subscription_dedupes_and_loses_session --locked -- --exact --test-threads=1
python3 -m unittest \
  scripts/perf/verify_memory_telemetry_coverage_test.py \
  scripts/perf/summarize_memory_diagnostic_test.py \
  scripts/perf/compare_memory_summaries_test.py
