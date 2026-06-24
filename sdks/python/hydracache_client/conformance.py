"""Python SDK conformance runner for the shared HydraCache manifest."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from .protocol import (
    PROTOCOL_VERSION,
    RepairAction,
    StableErrorCode,
    SubscriptionWatermarkTracker,
    stable_error_retryable,
)

DEFAULT_MANIFEST = (
    Path(__file__).resolve().parents[3]
    / "crates"
    / "hydracache-client"
    / "tests"
    / "fixtures"
    / "conformance"
    / "client_v1.json"
)


def load_manifest(path: Path = DEFAULT_MANIFEST) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def validate_manifest(manifest: dict[str, Any]) -> None:
    if manifest["protocol_version"] != PROTOCOL_VERSION:
        raise AssertionError("protocol version mismatch")

    scenario_ids = {scenario["id"] for scenario in manifest["scenarios"]}
    required = {
        "version-handshake-v1",
        "get-put-invalidate-round-trip",
        "near-cache-b1-repair",
        "deadline-retry-idempotency",
        "quota-backpressure-errors",
        "residency-denied-error",
    }
    missing = required - scenario_ids
    if missing:
        raise AssertionError(f"manifest missing scenarios: {sorted(missing)}")

    for entry in manifest["errors"]:
        code = StableErrorCode(entry["code"])
        if entry["retryable"] != stable_error_retryable(code):
            raise AssertionError(f"retryability mismatch for {code.value}")


def validate_near_cache_repair() -> None:
    tracker = SubscriptionWatermarkTracker()
    actions = [
        tracker.on_watermark(1, 1),
        tracker.on_watermark(1, 2),
        tracker.on_watermark(1, 4),
        tracker.on_watermark(2, 1),
    ]
    expected = [
        RepairAction.CLEAR_PARTITION,
        RepairAction.APPLY,
        RepairAction.INVALIDATE_CONSERVATIVELY,
        RepairAction.CLEAR_PARTITION,
    ]
    if actions != expected:
        raise AssertionError(f"near-cache repair mismatch: {actions}")


def run(manifest_path: Path = DEFAULT_MANIFEST) -> None:
    manifest = load_manifest(manifest_path)
    validate_manifest(manifest)
    validate_near_cache_repair()


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    args = parser.parse_args()
    run(args.manifest)


if __name__ == "__main__":
    main()
