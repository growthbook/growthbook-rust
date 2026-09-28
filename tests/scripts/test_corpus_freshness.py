"""Regression checks for corpus comparison and snapshot integrity."""

import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import check_corpus_freshness as checker


class CorpusFreshnessTests(unittest.TestCase):
    def test_new_nested_suites_are_not_silently_ignored(self):
        upstream = {"newCapability": {"feature": [["new case", True]]}}
        missing, _, _ = checker._diff(upstream, {})
        self.assertEqual(missing["newCapability.feature"], ["new case"])

    def test_changed_bodies_and_duplicate_counts_fail(self):
        upstream = {"feature": [["same", True], ["same", True]]}
        for local in [{"feature": [["same", True]]}, {"feature": [["same", True], ["same", False]]}]:
            _, drift, _ = checker._diff(upstream, local)
            self.assertEqual(drift["feature"], ["same"])

    def test_numeric_case_identifiers_are_compared(self):
        _, drift, _ = checker._diff({"getEqualWeights": [[2, [0.5, 0.5]]]}, {"getEqualWeights": [[2, [1, 0]]]})
        self.assertEqual(drift["getEqualWeights"], ["2"])

    def test_modified_snapshot_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            cases = Path(directory) / "cases.json"
            source = Path(directory) / "source.json"
            raw = b'{"specVersion":"0.9.0","feature":[]}'
            cases.write_bytes(raw)
            source.write_text(json.dumps({"specVersion": "0.9.0", "sha256": hashlib.sha256(raw).hexdigest()}))
            with patch.object(checker, "LOCAL_CASES", cases), patch.object(checker, "SOURCE", source):
                self.assertEqual(checker._load_local_cases()["feature"], [])
                cases.write_bytes(raw + b"\n")
                with self.assertRaisesRegex(ValueError, "checksum"):
                    checker._load_local_cases()


if __name__ == "__main__":
    unittest.main()
