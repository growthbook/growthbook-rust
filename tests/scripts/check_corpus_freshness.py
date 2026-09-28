#!/usr/bin/env python3
"""Verify the pinned cases.json and compare it against the upstream SDK corpus.

Rust additions and execution exclusions never suppress missing or changed base
cases. Tests use only checked-in files; this freshness check optionally fetches
newer upstream data. A fetch failure warns and skips, as in the existing CI job.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import urllib.error
import urllib.request
from collections import Counter
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
LOCAL_CASES = REPO_ROOT / "tests" / "cases" / "cases.json"
SOURCE = LOCAL_CASES.with_name("source.json")
DEFAULT_JS_URL = "https://raw.githubusercontent.com/growthbook/growthbook/main/packages/sdk-js/test/cases.json"


def _load_local_cases() -> dict:
    raw = LOCAL_CASES.read_bytes()
    source = json.loads(SOURCE.read_text())
    if hashlib.sha256(raw).hexdigest() != source["sha256"]:
        raise ValueError("cases.json does not match its recorded upstream checksum; update the complete snapshot and source.json together")
    cases = json.loads(raw)
    if cases.get("specVersion") != source["specVersion"]:
        raise ValueError("specVersion does not match source.json")
    return cases


def _fetch_js_cases(source: str) -> dict:
    if source.startswith(("http://", "https://")):
        request = urllib.request.Request(source, headers={"User-Agent": "growthbook-rust-corpus-check"})
        with urllib.request.urlopen(request, timeout=20) as response:
            return json.loads(response.read().decode("utf-8"))
    return json.loads(Path(source).read_text())


def _suites(corpus: dict, prefix: str = "") -> dict[str, list]:
    suites = {}
    for key, value in corpus.items():
        path = f"{prefix}.{key}" if prefix else key
        if isinstance(value, list):
            suites[path] = value
        elif isinstance(value, dict):
            suites.update(_suites(value, path))
    return suites


def _signatures(cases: list) -> dict[str, Counter]:
    grouped = {}
    for case in cases:
        if not isinstance(case, list) or not case:
            raise ValueError("Every corpus case must be a nonempty array")
        # Most suites have string names; getEqualWeights uses a numeric input.
        name = case[0] if isinstance(case[0], str) else json.dumps(case[0], sort_keys=True)
        signature = json.dumps(case[1:], sort_keys=True, separators=(",", ":"))
        grouped.setdefault(name, Counter())[signature] += 1
    return grouped


def _diff(upstream: dict, local: dict) -> tuple[dict, dict, dict]:
    missing, drift, extras = {}, {}, {}
    upstream_suites, local_suites = _suites(upstream), _suites(local)
    for suite in sorted(upstream_suites.keys() | local_suites.keys()):
        expected = _signatures(upstream_suites.get(suite, []))
        actual = _signatures(local_suites.get(suite, []))
        missing[suite] = [name for name in expected if name not in actual]
        drift[suite] = [name for name in expected if name in actual and expected[name] != actual[name]]
        extras[suite] = [name for name in actual if name not in expected]
    return missing, drift, extras


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--js-source", default=os.environ.get("GB_JS_CASES_URL", DEFAULT_JS_URL), help="Upstream URL or local path; defaults to SDK JS main")
    parser.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    args = parser.parse_args(argv)
    try:
        local = _load_local_cases()
    except (OSError, ValueError, KeyError) as error:
        print(f"corpus check error: {error}", file=sys.stderr)
        return 2
    try:
        upstream = _fetch_js_cases(args.js_source)
    except (OSError, ValueError) as error:
        print(f"WARNING: corpus freshness check skipped — could not fetch JS cases: {error}")
        return 0
    try:
        missing, drift, extras = _diff(upstream, local)
    except (ValueError, AttributeError) as error:
        print(f"corpus check error: {error}", file=sys.stderr)
        return 2
    failed = any(missing.values()) or any(drift.values())
    if args.json:
        print(json.dumps({"js_specVersion": upstream.get("specVersion"), "rust_specVersion": local.get("specVersion"), "missing_actionable": missing, "drift_actionable": drift, "extras": extras}, indent=2))
    else:
        print("=== Corpus freshness check (vendored cases.json vs JS SDK) ===")
        print(f"  JS specVersion: {upstream.get('specVersion')}")
        print(f"  Vendored specVersion: {local.get('specVersion')}")
        if upstream.get("specVersion") != local.get("specVersion"):
            print("  Version labels differ; case-level comparison follows.")
        print(f"{'DRIFT' if failed else 'OK'}: {sum(map(len, missing.values()))} missing, {sum(map(len, drift.values()))} changed, {sum(map(len, extras.values()))} additional vendored cases")
        for label, findings in [("Missing (fails CI)", missing), ("Changed (fails CI)", drift), ("Additional vendored cases (informational)", extras)]:
            for suite, names in findings.items():
                if names:
                    print(f"\n{label}: {suite}")
                    for name in names:
                        print(f"  - {name}")
    return int(failed)


if __name__ == "__main__":
    sys.exit(main())
