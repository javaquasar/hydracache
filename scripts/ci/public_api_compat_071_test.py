import unittest

from public_api_compat_071 import classify, publishable_packages, validate_manifest


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
                {"id": "public-id", "name": "public", "publish": None},
                {"id": "private-id", "name": "private", "publish": []},
                {"id": "outside-id", "name": "outside", "publish": None},
            ],
        }
        self.assertEqual(publishable_packages(metadata), {"public"})


if __name__ == "__main__":
    unittest.main()
