"""Regression checks for corpus comparison and snapshot integrity."""

import contextlib
import copy
import hashlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import check_corpus_freshness as checker


class CorpusFreshnessTests(unittest.TestCase):
    def run_checker(self, args, upstream=None, error=None):
        with (
            patch.dict(checker.os.environ, {}, clear=True),
            patch.object(checker, "_fetch_js_cases", return_value=upstream, side_effect=error) as fetch,
            contextlib.redirect_stdout(io.StringIO()),
            contextlib.redirect_stderr(io.StringIO()),
        ):
            result = checker.main(args)
        return result, fetch.call_args

    def test_default_verifies_recorded_commit_and_checksum(self):
        source = json.loads(checker.SOURCE.read_text())
        result, call = self.run_checker([], checker._load_local_cases())
        self.assertEqual(result, 0)
        self.assertEqual(call.args[0], f"https://raw.githubusercontent.com/{source['repository']}/{source['commit']}/{source['path']}")
        self.assertEqual(call.kwargs["expected_sha256"], source["sha256"])

    def test_scheduled_mode_detects_new_upstream_cases(self):
        upstream = copy.deepcopy(checker._load_local_cases())
        upstream["evalCondition"].append(["new upstream case", {}, {}, True])
        result, call = self.run_checker(["--upstream-main"], upstream)
        self.assertEqual(result, 1)
        self.assertIn("/main/", call.args[0])
        self.assertIsNone(call.kwargs["expected_sha256"])

    def test_explicit_source_remains_available(self):
        result, call = self.run_checker(["--js-source", "/tmp/cases.json"], checker._load_local_cases())
        self.assertEqual(result, 0)
        self.assertEqual(call.args[0], "/tmp/cases.json")
        self.assertIsNone(call.kwargs["expected_sha256"])

    def test_fetch_failures_do_not_pass_validation(self):
        for args in [[], ["--upstream-main"]]:
            result, _ = self.run_checker(args, error=OSError("offline"))
            self.assertEqual(result, 2)

    def test_pinned_download_must_match_exact_bytes(self):
        raw = b'{"feature": []}'
        checksum = hashlib.sha256(raw).hexdigest()
        with patch.object(checker.urllib.request, "urlopen", return_value=io.BytesIO(raw)):
            self.assertEqual(checker._fetch_js_cases("https://example.com/cases.json", checksum), {"feature": []})
        with patch.object(checker.urllib.request, "urlopen", return_value=io.BytesIO(raw + b"\n")):
            with self.assertRaisesRegex(ValueError, "pinned upstream file"):
                checker._fetch_js_cases("https://example.com/cases.json", checksum)

    def test_mutable_ref_is_not_a_pin(self):
        source = json.loads(checker.SOURCE.read_text())
        source["commit"] = "main"
        with self.assertRaisesRegex(ValueError, "full upstream commit SHA"):
            checker._source_url(source)

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
