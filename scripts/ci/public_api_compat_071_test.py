import unittest

from public_api_compat_071 import (
    classify,
    proc_macro_only_packages,
    profile_arguments,
    publishable_packages,
    validate_manifest,
)


def manifest():
    return {
        "schema_version": 1,
        "release": "0.71",
        "baseline_tag": "v0.70.0",
        "baseline_commit": "75719b0bf5de2250cf4eb16a30073dd7429538e3",
        "tool": "cargo-semver-checks",
        "tool_version": "0.49.0",
        "profiles": ["default", "all-features"],
        "packages": ["hydracache", "hydracache-core"],
        "profile_overrides": {
            "hydracache": {
                "all-features": {
                    "mode": "explicit-features",
                    "features": ["durable-values", "testing"],
                    "reason": "allocator selectors are mutually exclusive",
                }
            }
        },
        "excluded_packages": [
            {"package": "hydracache-macros", "reason": "proc-macro-only target"}
        ],
    }


class PublicApiCompatTest(unittest.TestCase):
    def test_manifest_and_exit_classification_are_fail_closed(self):
        self.assertEqual(validate_manifest(manifest()), [])
        self.assertEqual(classify(0), "compatible")
        self.assertEqual(classify(100), "semver-violation")
        self.assertEqual(classify(101), "analysis-or-tool-failure")
        self.assertEqual(classify(9), "analysis-or-tool-failure")

    def test_missing_profile_and_duplicate_package_are_rejected(self):
        value = manifest()
        value["profiles"] = ["default"]
        value["packages"] = ["hydracache", "hydracache"]
        found = validate_manifest(value)
        self.assertTrue(any("profile" in problem for problem in found))
        self.assertTrue(any("package" in problem for problem in found))

    def test_publishable_inventory_excludes_private_workspace_members(self):
        metadata = {
            "workspace_members": ["public-id", "private-id"],
            "packages": [
                {
                    "id": "public-id",
                    "name": "public",
                    "publish": None,
                    "targets": [{"kind": ["lib"]}],
                },
                {
                    "id": "private-id",
                    "name": "private",
                    "publish": [],
                    "targets": [{"kind": ["lib"]}],
                },
                {
                    "id": "outside-id",
                    "name": "outside",
                    "publish": None,
                    "targets": [{"kind": ["lib"]}],
                },
                {
                    "id": "macro-id",
                    "name": "macros",
                    "publish": None,
                    "targets": [{"kind": ["proc-macro"]}],
                },
            ],
        }
        self.assertEqual(publishable_packages(metadata), {"public"})

    def test_profiles_are_explicit_and_never_enable_conflicting_features_implicitly(self):
        value = manifest()
        self.assertEqual(profile_arguments(value, "hydracache-core", "default"), ["--default-features"])
        self.assertEqual(profile_arguments(value, "hydracache-core", "all-features"), ["--all-features"])
        self.assertEqual(
            profile_arguments(value, "hydracache", "all-features"),
            ["--only-explicit-features", "--features", "durable-values,testing"],
        )

    def test_only_proc_macro_targets_are_identified_as_excludable(self):
        metadata = {
            "workspace_members": ["lib-id", "macro-id", "mixed-id"],
            "packages": [
                {"id": "lib-id", "name": "lib", "targets": [{"kind": ["lib"]}]},
                {"id": "macro-id", "name": "macros", "targets": [{"kind": ["proc-macro"]}]},
                {
                    "id": "mixed-id",
                    "name": "mixed",
                    "targets": [{"kind": ["proc-macro"]}, {"kind": ["lib"]}],
                },
            ],
        }
        self.assertEqual(proc_macro_only_packages(metadata), {"macros"})


if __name__ == "__main__":
    unittest.main()
