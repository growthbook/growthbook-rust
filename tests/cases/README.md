# Shared conformance cases

- `cases.json` is an exact, unedited copy of the JavaScript SDK's base corpus.
- `source.json` records its repository, commit, path, spec version, and SHA-256.
- `rust.json` contains only Rust-specific additions. The loader rejects names
  that shadow upstream cases instead of overriding upstream expectations.
- `exclusions.json` lists unsupported suites/cases with reasons. Exclusions
  affect execution only; excluded cases remain in `cases.json` and are checked
  for freshness.
- `mod.rs` combines the files offline for unit and integration tests. `load()`
  returns every case; `active()` applies execution exclusions and rejects stale
  exclusions. No downloaded or generated combined file is needed to run tests.

Contextual bandits are unsupported. Their 35 dedicated cases and four related
feature cases are excluded from normal execution and available through the
ignored `growthbook::test::evaluate_contextual_bandits` test. Ordinary experiment
and unknown-field tolerance cases still run. When adding support, remove the
exclusions/ignore and extend result validation to cover bandit metadata.

The vendored snapshot also contains saved-group v2 suites. This maintenance
branch excludes them until the separate saved-group implementation adds its
runners. Corpus contents and `specVersion` do not declare SDK capabilities.

## Updating the snapshot

Copy the complete upstream `packages/sdk-js/test/cases.json` from a specific
commit, preserving its bytes. Update `source.json` with that commit, its
`specVersion`, and the output of `shasum -a 256 tests/cases/cases.json`. Never
hand-edit the upstream file or change its version independently of its cases.
Keep local cases in `rust.json`; if upstream adopts one, remove the local copy.

Run the Rust tests and compare the snapshot with upstream:

```sh
cargo test --locked
cargo +1.75.0 test --locked
python3 tests/scripts/check_corpus_freshness.py
python3 -m unittest discover -s tests/scripts -p 'test_*.py'
```

The freshness checker validates the local checksum first, then compares every
suite (including nested and unsupported suites) with JS `main`. Missing or
changed upstream cases fail; extra cases from a newer pinned snapshot are
informational. Rust additions and execution exclusions cannot hide drift.
An upstream fetch failure remains a warning, matching the existing CI policy.

To check against a local checkout instead of fetching:

```sh
python3 tests/scripts/check_corpus_freshness.py --js-source ../growthbook/packages/sdk-js/test/cases.json
```
