# Saved group v2 performance and runtime checks

Production code compared: `157a5f7` (before this PR) versus `e1afd81` (current PR). The benchmark and tests are uncommitted additions; no production optimization was needed.

## Result

No major regression was observed in the measured, previously supported paths. Across 41 comparable measurements, changes in the median of three run medians ranged from -0.9% to +7.0%. All timed operations completed without errors. This is a local benchmark result, not a guarantee for every production workload.

The first current run had a transient legacy-list loading slowdown (about 41% for the smaller offline load); the other two current runs were close to baseline. Raw run medians are retained in [the result data](saved-group-v2-results.json) so the variability is visible. No run was discarded.

## Sync and async behavior

There are no separate sync and async Rust SDK variants. `GrowthBook::check` and the client evaluation methods are synchronous. Client setup and refresh are async; HTTP loading and automatic refresh use Tokio. There is no blocking HTTP client API.

`tests/server_saved_group_payloads.rs` now checks direct evaluation without any runtime and client evaluation after dropping its setup runtime. Each covers 360 comparisons (three plaintext payload formats × six users × 20 features). The existing async initial-load and refresh tests cover 1,440 comparisons across six plaintext/encrypted formats, plus caching and failure behavior. All seven tests passed on stable and Rust 1.75.0.

## Method

- Apple M5 Pro, arm64, macOS 26.6.2; Rust 1.98.1; optimized bench profile with default features. Same machine, toolchain, lockfile, harness, and fixture in both revisions.
- Three interleaved runs per revision: before/current, current/before, before/current. Each measurement calibrates its iteration count, then takes nine approximately 40 ms batch samples. Tables compare the median of each revision’s three run medians.
- Compilations finished before timing; no other agent benchmark or build ran concurrently. Ordinary workstation scheduling, allocator state, and background activity were not controlled.
- Synthetic workloads: 100 or 1,000 background feature definitions plus one targeted feature; legacy/v2 lists contain 1,000 or 10,000 string members. Evaluations target the last member, so membership checks scan the full list. V2 condition groups add two reference levels before the list.
- Server fixtures are the checked-in production-builder outputs with 20 features. Evaluation timings use the `list` feature and the first user. These do not measure every rule or every possible nested condition.
- `decode_json`: bytes to `GrowthBookResponse`, including destruction. `load_offline`: decode plus manual client construction and destruction. `refresh_cached`: real async refresh from a preloaded cache, including decryption when applicable. `refresh_http`: real async refresh over a local mock HTTP server with cache TTL zero.
- `evaluate_core`: `GrowthBook::check` on an already-loaded configuration. `evaluate_client`: public `feature_result`, including its existing configuration/attribute cloning. Tokio runtime construction and fixture setup are outside timed sections; operations’ allocation and cleanup costs are included.
- V2 and encrypted saved-group loading have no working equivalent in the baseline, so they are reported separately from regression comparisons. Encrypted inline-feature loading is comparable and is included below.

## Before/after timings

All times are microseconds per operation. Positive percentages mean slower. “100/1000” and “1000/10000” in workload names mean background features/list members.

| Workload / operation | Before (µs) | Current (µs) | Change |
| --- | ---: | ---: | ---: |
| `synthetic-100-1000-plain/decode_json` | 7.302 | 7.375 | +1.0% |
| `synthetic-100-1000-plain/load_offline` | 9.346 | 9.485 | +1.5% |
| `synthetic-100-1000-plain/refresh_cached` | 1.801 | 1.863 | +3.5% |
| `synthetic-100-1000-plain/evaluate_core` | 0.208 | 0.213 | +2.6% |
| `synthetic-100-1000-plain/evaluate_client` | 1.950 | 2.087 | +7.0% |
| `synthetic-100-1000-plain/refresh_http` | 97.156 | 98.530 | +1.4% |
| `synthetic-100-1000-legacy/decode_json` | 30.975 | 31.327 | +1.1% |
| `synthetic-100-1000-legacy/load_offline` | 57.504 | 59.638 | +3.7% |
| `synthetic-100-1000-legacy/refresh_cached` | 40.274 | 41.906 | +4.1% |
| `synthetic-100-1000-legacy/evaluate_core` | 2.739 | 2.717 | -0.8% |
| `synthetic-100-1000-legacy/evaluate_client` | 17.415 | 17.300 | -0.7% |
| `synthetic-100-1000-legacy/refresh_http` | 173.561 | 174.929 | +0.8% |
| `synthetic-1000-10000-plain/decode_json` | 74.690 | 75.309 | +0.8% |
| `synthetic-1000-10000-plain/load_offline` | 91.525 | 92.259 | +0.8% |
| `synthetic-1000-10000-plain/refresh_cached` | 17.214 | 17.494 | +1.6% |
| `synthetic-1000-10000-plain/evaluate_core` | 0.211 | 0.218 | +3.2% |
| `synthetic-1000-10000-plain/evaluate_client` | 16.683 | 16.700 | +0.1% |
| `synthetic-1000-10000-plain/refresh_http` | 215.989 | 214.039 | -0.9% |
| `synthetic-1000-10000-legacy/decode_json` | 310.659 | 308.199 | -0.8% |
| `synthetic-1000-10000-legacy/load_offline` | 572.661 | 577.370 | +0.8% |
| `synthetic-1000-10000-legacy/refresh_cached` | 397.524 | 398.969 | +0.4% |
| `synthetic-1000-10000-legacy/evaluate_core` | 23.558 | 24.109 | +2.3% |
| `synthetic-1000-10000-legacy/evaluate_client` | 169.359 | 169.009 | -0.2% |
| `synthetic-1000-10000-legacy/refresh_http` | 954.902 | 962.145 | +0.8% |
| `server-inline/decode_json` | 12.491 | 12.662 | +1.4% |
| `server-inline/load_offline` | 17.061 | 17.457 | +2.3% |
| `server-inline/refresh_cached` | 4.112 | 4.228 | +2.8% |
| `server-inline/evaluate_core` | 0.731 | 0.753 | +3.0% |
| `server-inline/evaluate_client` | 4.888 | 5.069 | +3.7% |
| `server-inline/refresh_http` | 108.963 | 109.280 | +0.3% |
| `server-inline-encrypted/decode_json` | 1.376 | 1.387 | +0.7% |
| `server-inline-encrypted/refresh_cached` | 21.869 | 21.817 | -0.2% |
| `server-inline-encrypted/evaluate_core` | 0.720 | 0.732 | +1.7% |
| `server-inline-encrypted/evaluate_client` | 4.953 | 5.024 | +1.4% |
| `server-inline-encrypted/refresh_http` | 111.534 | 117.632 | +5.5% |
| `server-referencesV1/decode_json` | 12.197 | 12.298 | +0.8% |
| `server-referencesV1/load_offline` | 16.960 | 17.161 | +1.2% |
| `server-referencesV1/refresh_cached` | 4.416 | 4.576 | +3.6% |
| `server-referencesV1/evaluate_core` | 0.580 | 0.590 | +1.9% |
| `server-referencesV1/evaluate_client` | 4.772 | 4.871 | +2.1% |
| `server-referencesV1/refresh_http` | 106.193 | 107.504 | +1.2% |

## New paths: current-only timings

These are medians from one nine-sample run per workload, not before/after regression percentages.

| Workload / operation | Current (µs) |
| --- | ---: |
| `synthetic-100-1000-v2-list/decode_json` | 29.019 |
| `synthetic-100-1000-v2-list/load_offline` | 55.666 |
| `synthetic-100-1000-v2-list/refresh_cached` | 39.326 |
| `synthetic-100-1000-v2-list/evaluate_core` | 2.739 |
| `synthetic-100-1000-v2-list/evaluate_client` | 17.114 |
| `synthetic-100-1000-v2-list/refresh_http` | 172.826 |
| `synthetic-100-1000-v2-condition/decode_json` | 30.335 |
| `synthetic-100-1000-v2-condition/load_offline` | 58.754 |
| `synthetic-100-1000-v2-condition/refresh_cached` | 41.485 |
| `synthetic-100-1000-v2-condition/evaluate_core` | 2.860 |
| `synthetic-100-1000-v2-condition/evaluate_client` | 18.433 |
| `synthetic-100-1000-v2-condition/refresh_http` | 191.384 |
| `synthetic-1000-10000-v2-list/decode_json` | 303.231 |
| `synthetic-1000-10000-v2-list/load_offline` | 564.523 |
| `synthetic-1000-10000-v2-list/refresh_cached` | 388.379 |
| `synthetic-1000-10000-v2-list/evaluate_core` | 23.795 |
| `synthetic-1000-10000-v2-list/evaluate_client` | 169.982 |
| `synthetic-1000-10000-v2-list/refresh_http` | 939.833 |
| `synthetic-1000-10000-v2-condition/decode_json` | 310.122 |
| `synthetic-1000-10000-v2-condition/load_offline` | 568.717 |
| `synthetic-1000-10000-v2-condition/refresh_cached` | 390.100 |
| `synthetic-1000-10000-v2-condition/evaluate_core` | 24.597 |
| `synthetic-1000-10000-v2-condition/evaluate_client` | 172.423 |
| `synthetic-1000-10000-v2-condition/refresh_http` | 932.279 |
| `server-referencesV2/decode_json` | 15.641 |
| `server-referencesV2/load_offline` | 23.355 |
| `server-referencesV2/refresh_cached` | 8.736 |
| `server-referencesV2/evaluate_core` | 0.572 |
| `server-referencesV2/evaluate_client` | 5.657 |
| `server-referencesV2/refresh_http` | 128.024 |
| `server-referencesV2-encrypted/decode_json` | 1.903 |
| `server-referencesV2-encrypted/refresh_cached` | 31.558 |
| `server-referencesV2-encrypted/evaluate_core` | 0.584 |
| `server-referencesV2-encrypted/evaluate_client` | 5.693 |
| `server-referencesV2-encrypted/refresh_http` | 121.455 |
| `server-referencesV1-encrypted/decode_json` | 1.498 |
| `server-referencesV1-encrypted/refresh_cached` | 23.389 |
| `server-referencesV1-encrypted/evaluate_core` | 0.582 |
| `server-referencesV1-encrypted/evaluate_client` | 4.764 |
| `server-referencesV1-encrypted/refresh_http` | 114.410 |

## Interpretation and limits

The client wrapper clones its stored configuration during each evaluation in both revisions. This explains why client evaluation scales with total payload size even when direct evaluation does not. For example, a plain rule with 1,000 background flags takes about 0.22 µs through the core and 16.7 µs through the client; this behavior predates the PR.

List membership is linear in the number of members. This benchmark exercises full scans, but not concurrent callers, sustained high request rates, allocation counts, peak memory, network latency, or very deep/cyclic reference chains. Those remain outside the performance conclusion. HTTP timings include local mock-server and scheduler overhead.

## Reproduction

```sh
cargo bench --locked --bench payload_evaluation > /tmp/current.jsonl
BENCH_V2=0 cargo bench --locked --bench payload_evaluation > /tmp/current-compatible.jsonl
```

To compare against the baseline, export `157a5f7` to a temporary directory, copy `benches/payload_evaluation.rs` and `tests/fixtures/saved_group_server_payloads.json` into it, and add the same `[[bench]]` entry from `Cargo.toml`. Compile both revisions with `--locked --no-run`, then run their executables separately with `BENCH_V2=0` in the order listed above. Preserve separate executable copies if sharing a Cargo target directory. The production sources in both revisions must remain unchanged.

For new paths only, use `BENCH_FILTER=v2`, `BENCH_FILTER=referencesV2`, and `BENCH_FILTER=referencesV1-encrypted` on the current executable. `BENCH_SAMPLES` and `BENCH_SAMPLE_MS` can extend the sampling duration.
