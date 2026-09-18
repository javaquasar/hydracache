import copy
import unittest

from memory_compat_receipt_071 import BASELINE_COMMIT, REQUIRED_CHECKS, canonical_digest, problems


CANDIDATE = "a" * 40
WORKFLOW = "b" * 40


def valid_receipt():
    receipt = {
        "schema_version": 1,
        "release": "0.71",
        "campaign_id": "campaign-1",
        "baseline_tag": "v0.70.0",
        "baseline_commit": BASELINE_COMMIT,
        "candidate_sha": CANDIDATE,
        "workflow_sha": WORKFLOW,
        "result": "success",
        "generated_at": "2026-09-14T00:00:00+00:00",
        "driver_sha256": "c" * 64,
        "binary_sha256": {"baseline": "d" * 64, "candidate": "e" * 64},
        "checks": sorted(REQUIRED_CHECKS),
    }
    receipt["receipt_sha256"] = canonical_digest(receipt)
    return receipt


class ReceiptTest(unittest.TestCase):
    def test_complete_receipt_is_accepted(self):
        self.assertEqual(problems(valid_receipt(), "campaign-1", CANDIDATE, WORKFLOW), [])

    def test_missing_check_and_mixed_identity_are_rejected(self):
        receipt = valid_receipt()
        receipt["checks"].pop()
        receipt["candidate_sha"] = "f" * 40
        receipt["receipt_sha256"] = canonical_digest(receipt)
        found = problems(receipt, "campaign-1", CANDIDATE, WORKFLOW)
        self.assertTrue(any("incomplete" in value for value in found))
        self.assertTrue(any("candidate_sha" in value for value in found))

    def test_tampered_or_identical_binary_receipt_is_rejected(self):
        receipt = copy.deepcopy(valid_receipt())
        receipt["binary_sha256"]["candidate"] = receipt["binary_sha256"]["baseline"]
        found = problems(receipt, "campaign-1", CANDIDATE, WORKFLOW)
        self.assertTrue(any("identical" in value for value in found))
        self.assertTrue(any("seal" in value for value in found))


if __name__ == "__main__":
    unittest.main()
