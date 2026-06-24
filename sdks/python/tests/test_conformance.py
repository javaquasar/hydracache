from hydracache_client.conformance import load_manifest, run
from hydracache_client.protocol import RepairAction, SubscriptionWatermarkTracker


def test_manifest_runner_passes_shared_contract() -> None:
    run()


def test_near_cache_repair_actions_match_b1_contract() -> None:
    tracker = SubscriptionWatermarkTracker()
    assert tracker.on_watermark(1, 1) == RepairAction.CLEAR_PARTITION
    assert tracker.on_watermark(1, 2) == RepairAction.APPLY
    assert tracker.on_watermark(1, 4) == RepairAction.INVALIDATE_CONSERVATIVELY
    assert tracker.on_watermark(2, 1) == RepairAction.CLEAR_PARTITION


def test_manifest_declares_python_sdk_supported() -> None:
    manifest = load_manifest()
    assert any(
        sdk["language"] == "python" and sdk["supported"]
        for sdk in manifest["sdks"]
    )
