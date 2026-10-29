from __future__ import annotations

import unittest

import performance_allocator_profile_074 as profile


def native(allocator: str) -> dict:
    metric = {"bytes": 1, "source": "test", "semantics": "test bytes"}
    if allocator == "system":
        return {
            "provider": "system-glibc-mallinfo2",
            "status": "partial-native-statistics",
            "allocated_or_live": metric,
            "active_or_committed": metric,
            "resident": None,
            "retained_or_reserved": metric,
            "arenas": None,
            "thread_caches": None,
            "unavailable": {"resident": "not exposed", "thread_caches": "not exposed"},
            "raw": {},
        }
    if allocator == "mimalloc":
        return {
            "provider": "mimalloc",
            "status": "partial-native-statistics",
            "allocated_or_live": None,
            "active_or_committed": metric,
            "resident": None,
            "retained_or_reserved": metric,
            "arenas": 1,
            "thread_caches": None,
            "unavailable": {
                "allocated_or_live": "not reliable",
                "resident": "not exposed",
                "thread_caches": "not exposed",
            },
            "raw": {},
        }
    return {
        "provider": "jemalloc",
        "status": "available",
        "allocated_or_live": metric,
        "active_or_committed": metric,
        "resident": metric,
        "retained_or_reserved": metric,
        "arenas": 1,
        "thread_caches": None,
        "unavailable": {"thread_caches": "not exposed"},
        "raw": None,
    }


def receipt(allocator: str = "system") -> dict:
    phases = []
    for phase, operations, entries in zip(
        profile.NO_PURGE_PHASES,
        profile.EXPECTED_OPERATIONS,
        profile.EXPECTED_ENTRIES,
        strict=True,
    ):
        process = {field: 1 for field in profile.PROCESS_FIELDS}
        process.update(
            {
                "source": "linux-procfs-and-getrusage",
                "private_commit_bytes": None,
                "unavailable": {"private_commit_bytes": "not exposed"},
            }
        )
        phases.append(
            {
                "phase": phase,
                "elapsed_ns": operations + 1,
                "completed_operations": operations,
                "logical_entries": entries,
                "process": process,
                "allocator_native": native(allocator),
            }
        )
    return {
        "schema_version": profile.SCHEMA_VERSION,
        "profile_id": profile.PROFILE_ID,
        "allocator": allocator,
        "allocator_feature": f"allocator-{allocator}",
        "target": "x86_64-linux",
        "mode": "no-purge",
        "workload_seed": 74_009,
        "cardinality": 16_384,
        "payload_bytes": 4_096,
        "steady_read_operations": 65_536,
        "idle_milliseconds": 2_000,
        "elapsed_ns": 1,
        "executable_bytes": 1,
        "phases": phases,
        "invariants": {
            "exact_phase_order": True,
            "exact_logical_cardinality": True,
            "payload_shape_preserved": True,
            "rss_used_as_native_substitute": False,
        },
        "promotable": False,
    }


class AllocatorProfileTests(unittest.TestCase):
    def test_schedule_is_balanced_and_retains_separate_purge_policy(self) -> None:
        schedule = profile.schedule_for(5)
        self.assertEqual(len(schedule), 20)
        for allocator in profile.ALLOCATORS:
            self.assertEqual(schedule.count((allocator, "no-purge")), 5)
        self.assertEqual(schedule.count(("mimalloc", "purge")), 5)
        self.assertEqual(schedule[:3], [("system", "no-purge"), ("mimalloc", "no-purge"), ("jemalloc", "no-purge")])

    def test_valid_system_receipt_passes(self) -> None:
        profile.validate_receipt(receipt(), "system", "no-purge")

    def test_target_drift_fails_closed(self) -> None:
        value = receipt()
        value["target"] = "aarch64-linux"
        with self.assertRaisesRegex(ValueError, "target changed"):
            profile.validate_receipt(value, "system", "no-purge")

    def test_rss_cannot_fill_allocator_native_resident(self) -> None:
        value = receipt()
        value["phases"][0]["allocator_native"]["resident"] = {
            "bytes": 1,
            "source": "proc.VmRSS",
            "semantics": "wrong substitution",
        }
        with self.assertRaisesRegex(ValueError, "resident was imputed"):
            profile.validate_receipt(value, "system", "no-purge")


if __name__ == "__main__":
    unittest.main()
