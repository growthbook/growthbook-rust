# Contributing Guide

We welcome all contributions!

This repo is the official GrowthBook SDK for Rust — a client library for
evaluating feature flags and running experiments in Rust applications.

## Requirements

- **Rust**, installed with [rustup](https://rust-lang.org/tools/install/).
  `rust-toolchain` selects stable for local development; `Cargo.toml` declares
  Rust 1.75.0 as the minimum supported version. See
  [Language version](#language-version).
- **A native compiler and linker**. On macOS, install the Xcode command line
  tools with `xcode-select --install` if Xcode is not already installed.
- **rustfmt and Clippy** for formatting and lint checks:

  ```sh
  rustup component add rustfmt clippy
  ```

- **Python 3** for the conformance corpus freshness check.
- **cargo-nextest and cargo-watch** — optional tools used by the Makefile:

  ```sh
  cargo install --locked cargo-nextest cargo-watch
  ```

For editor support, install your editor's Rust integration and optionally the
`rust-analyzer` toolchain component with `rustup component add rust-analyzer`.

## Getting started

Fork the repo, or clone directly if you have write access:

```sh
git clone git@github.com:growthbook/growthbook-rust.git
cd growthbook-rust
cargo build --locked
cargo test --locked
```

Cargo downloads dependencies on the first build. `--locked` uses the committed
`Cargo.lock` without updating dependency versions. Get a passing baseline before
you start writing code.

Tests use local mock servers; no GrowthBook account, SDK key, `.env` file, or
separate service is required. Allow local socket access when running tests in a
sandbox.

If a terminal opened before installing Rust cannot find `cargo`, restart the
terminal or run `source "$HOME/.cargo/env"` on macOS/Linux.

## Writing code

### Layout

| Path | Contents |
| --- | --- |
| `src/client.rs` | Client, builder, callbacks, and feature refresh |
| `src/growthbook.rs`, `src/feature/` | Feature and experiment evaluation |
| `src/condition/` | Targeting conditions and comparisons |
| `src/hash.rs`, `src/range/`, `src/namespace/` | Hashing and bucketing |
| `src/gateway.rs`, `src/cache.rs` | Feature fetching and caching |
| `src/model_public.rs`, `src/dto.rs` | Public models and serialized data |
| `src/sticky_bucket.rs` | Sticky bucketing services |
| `tests/` | Integration tests, shared helpers, and conformance data |
| `examples/client/` | Standalone example crate using the local SDK |

### Public API and SDK parity

`src/lib.rs` declares the public modules. Treat changes to exposed types,
traits, and methods as changes to the SDK's public API. Client configuration
uses `GrowthBookClientBuilder`; follow the existing builder methods when adding
configuration.

GrowthBook's SDKs should make identical decisions given identical inputs. Use
the JavaScript SDK as the reference implementation, add regression tests for
behavior changes, and call out any deliberate differences in the PR.

### Language version

Rust 1.75.0 is the minimum supported Rust version (MSRV). CI builds and tests
with both 1.75.0 and stable. Check compatibility locally, especially when
changing dependencies or using newer language or standard library features:

```sh
rustup toolchain install 1.75.0 --profile minimal
cargo +1.75.0 build --locked
cargo +1.75.0 test --locked
```

Keep `Cargo.lock` readable by Rust 1.75.0; regenerating it with a newer Cargo
can introduce an unsupported lockfile format or dependencies with a higher
MSRV. Discuss raising the minimum version before including it in a change.

## Testing

Unit tests live alongside the implementation under `src/`. Integration tests
live in `tests/`, with shared mock-server helpers in `tests/commons.rs`.

```sh
cargo test --locked                              # All tests, including doc tests
cargo test --locked --test manual_features       # One integration test file
cargo test --locked test_offline_mode_manual_features # Filter by test name
cargo test --locked -- --nocapture               # Show test output
```

The Makefile provides nextest and watch mode after installing the optional
tools listed above:

```sh
make test_oneshot        # Run tests once with nextest
make test               # Rerun tests when files change; Ctrl-C to stop
make test FILTER=is_on  # Watch tests matching a name
```

Use `cargo test --locked` for the full CI test command, including doc tests.
Add a regression test with a bug fix that fails before the fix and passes after.

### Synchronous evaluation and async loading

This crate has one SDK implementation. `GrowthBook::check` and the client
methods `feature_result`, `is_on`, and `is_off` are synchronous. Client `build`,
`refresh`, and `try_refresh` are async; network loading and automatic refresh
use Tokio. There is no separate blocking HTTP client.

Evaluation does not require a running Tokio runtime after setup. The server
payload tests cover direct synchronous evaluation without a runtime, client
evaluation after its setup runtime is dropped, and async loading/refresh:

```sh
cargo test --locked --test server_saved_group_payloads
```

### Performance benchmarks

Run the benchmark harness in release mode; it uses the existing dependencies:

```sh
cargo bench --locked --bench payload_evaluation > /tmp/growthbook-benchmark.jsonl
```

It measures JSON decoding, offline client loading, cached refresh, localhost
HTTP refresh, direct `GrowthBook::check`, and client `feature_result`. Workloads
include server-generated payloads and synthetic payloads with 100 or 1,000
background features plus one targeted feature, and lists of 1,000 or 10,000
members. Synthetic evaluations target the last list member to exercise a full
membership scan. Setup and fixture construction are outside the timed sections;
allocations, cloning, and dropping the operation's results are included.

`BENCH_FILTER` selects workload names (for example, `server-referencesV2`).
`BENCH_SAMPLES` and `BENCH_SAMPLE_MS` control sampling (defaults: 9 samples of
approximately 40 ms each, after calibration). Each JSON result includes raw
sample timings and an error count; results with errors are not valid latency
comparisons. `BENCH_V2=0` limits the harness to paths supported before the saved
group v2 change, including plaintext v1 and encrypted inline features.

For comparisons, use the same harness and fixtures in both revisions, compile
before measuring, and alternate multiple runs on the same idle machine. Do not
compare debug builds or run compilations concurrently with measurements.
Local HTTP results include mock-server and scheduling overhead, not production
network latency. These measurements are observational, not CI timing gates.

### Conformance corpus

`tests/cases/cases.json` is an exact, pinned copy of the upstream SDK corpus.
Rust-specific additions live in `tests/cases/rust.json`, and unsupported suites
or cases are listed with reasons in `tests/cases/exclusions.json`. The shared
loader combines these files offline without changing upstream expectations.
See [the corpus guide](tests/cases/README.md) for snapshot update instructions.

Contextual bandits remain unsupported and excluded from normal execution.
Their cases are retained in the upstream file and available through the ignored
`growthbook::test::evaluate_contextual_bandits` test. Saved-group v2 suites run
in `tests/saved_group_references_v2.rs`.

CI validates the pinned checksum and compares every upstream suite with the
JavaScript SDK's current `main` branch:

```sh
python3 tests/scripts/check_corpus_freshness.py
python3 -m unittest discover -s tests/scripts -p 'test_*.py'
```

To compare against a local copy instead of fetching:

```sh
python3 tests/scripts/check_corpus_freshness.py --js-source /path/to/cases.json
```

Missing or changed upstream cases fail. Rust additions and execution exclusions
cannot hide drift. Investigate freshness failures even when local Rust tests
pass; they can reflect new upstream cases for unsupported SDK capabilities.

### Server-generated payloads

`tests/fixtures/saved_group_server_payloads.json` is generated by the GrowthBook
server's production payload builder. The tests load the responses through the
Rust HTTP client and compare them with JavaScript SDK results across inline,
v1, and v2 saved groups, both plaintext and encrypted. They also cover refresh,
cached payloads, and decryption failures.

Run the checked-in fixtures without a server checkout:

```sh
cargo test --locked --test server_saved_group_payloads
```

To regenerate them, use Node.js and a sibling GrowthBook checkout with its
dependencies installed and saved group references v2 implemented:

```sh
node tests/scripts/generate_saved_group_payloads.cjs ../growthbook
cargo test --locked --test server_saved_group_payloads
```

The generator uses server and JavaScript SDK source files directly. It uses
synthetic data and a test encryption key, records the server commit in the
fixture, and requires no running server, database, or account credentials.
It does not modify the server checkout or its capability declarations.

## Code quality

Run the same formatting and lint checks as CI:

```sh
cargo fmt --all -- --check
cargo clippy --locked -- -D warnings
```

Use `make fmt` to apply formatting, and review the diff for unrelated changes.
`rustfmt.toml` holds the formatting configuration.

`make clippy` is stricter than CI: it checks all targets and features and also
denies `clippy::unwrap_used`. It may report existing issues in test code that
the CI command does not check.

## Opening pull requests

1. Branch from `main` and keep the diff focused.
2. Describe the problem, resulting behavior, and any public API or SDK parity
   implications. Discuss substantial API or behavior changes in an issue first.
3. Run `cargo build --locked`, `cargo test --locked`, and the code quality
   commands above. Check Rust 1.75.0 compatibility and run the corpus freshness
   check when changing evaluation behavior or conformance data.
4. Open the PR against `main` and explain how you tested the change.

CI runs builds and tests on Rust 1.75.0 and stable, formatting and Clippy on
stable, and the corpus freshness check. Its configuration is in
`.github/workflows/ci.yml`.

## Releasing

Maintainers handle releases through `.github/workflows/release-please.yml`.
Release Please prepares version and changelog updates from conventional commits;
merging a release PR creates the release and triggers publication to crates.io
after tests pass. Contributors do not need to bump versions or edit the
changelog for ordinary changes.

## Getting help

- [Open an issue](https://github.com/growthbook/growthbook-rust/issues)
- [Rust API documentation](https://docs.rs/growthbook-rust)
- [GrowthBook Slack community](https://slack.growthbook.io?ref=contributing)
