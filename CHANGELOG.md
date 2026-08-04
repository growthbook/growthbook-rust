# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0]

This release brings feature-flag evaluation in line with the GrowthBook JS and 
other SDKs (the cross-SDK source of truth).

### ⚠️ Breaking Changes

#### API (compile-time)
- `dto::GrowthBookFeatureRule` is no longer a `#[serde(untagged)]` enum. It is
  now a struct with a `kind: GrowthBookFeatureRuleKind` field (plus
  `parent_conditions`). Code that matched on or constructed
  `GrowthBookFeatureRule::Force(..)` / `Experiment(..)` / `Rollout(..)` /
  `Parent(..)` / `Empty(..)` must migrate to the new `kind` field.

#### Behavioral (evaluation results may change after upgrade)
- **`parentConditions` now apply to force, rollout, and experiment rules.**
  They were previously dropped on force rules, so the force value fired
  unconditionally regardless of its prerequisites.
- **Non-gating vs gating prerequisite failures now differ.** A failed parent
  with `gate: false` skips only that rule and continues to the next; a failed
  parent with `gate: true` returns a `prerequisite` result.
- **Experiment-rule `condition` is now enforced** (evaluated after forced
  variations, matching JS `runExperiment`).
- **`$eq` / `$ne` no longer coerce across JS types** — `5` no longer equals the
  string `"5"`, and `true` no longer equals `1`. Numbers still compare by value
  (`5` equals `5.0`), and `$lt` / `$gt` keep full coercion, all matching JS.
- **`{}` is now typed `"object"`, not `"null"`.** A `{ x: {} }` condition now
  requires the attribute to be an actual empty object.
- **A non-string `$regex` pattern** (e.g. `{ x: { $regex: 5 } }`) now matches
  nothing (previously it matched everything).

### 🚀 Features
- **Saved-group targeting**: Implemented the `$inGroup` / `$notInGroup`
  operators and `savedGroups` support, including on the manual features builder
  path (`GrowthBookClientBuilder`).

### 🐛 Bug Fixes
- Honor `parentConditions` and the `gate` flag on every feature rule (see
  Breaking Changes for the behavioral impact).
- `$eq` / `$ne` strict scalar equality, `$type` on empty objects, and empty
  `{}` handling now match the JS SDK.

### 🧰 Internal / Tooling
- Declared the MSRV as Rust 1.75.0 (`rust-version` in `Cargo.toml`) and
  re-encoded `Cargo.lock` to lockfile v3 so the 1.75 CI job can parse it; added
  `--locked` to the build/test CI steps.
- Added a corpus-freshness CI check and caught `tests/all_cases.json` up to the
  JS SDK conformance suite.

## [0.1.1]

### 🚀 Features
- **Case-insensitive operators**: Added support for case-insensitive operators `regexi`, `notRegexi`, `ini`, `nini` and `alli` in feature flag conditions.
- Tracing made optional. 

## [0.1.0]

### 🚀 Features
- **Sticky Bucketing**: Added support for sticky bucketing to ensure users persist in their assigned variations.
- Updated scenario test spec to `0.7.1`.

## [0.0.4] - 2025-12-17

### 🚀 Features
- **Offline Mode**: Support for initializing the client with manual features and no valid URL/Key.
- **CI Modernization**: Updated to use `dtolnay/rust-toolchain`, strict version pinning (1.75.0, stable, beta), and formatting/clippy checks.
- **Verification**: Added `GrowthBookClientTrait` and improved testability.

### 🐛 Bug Fixes
- **Manual Features**: Fixed an issue where `client.refresh()` was called unconditionally, overwriting manual features.
- **Lints**: Resolved various clippy warnings and formatting issues.

## [0.0.3] - 2025-11-25

### 🚀 Features
- **Encrypted Features**: Added support for decrypting encrypted feature flags using AES-CBC.
- **Dependencies**: Added `aes`, `cbc`, and `base64` dependencies.

## [0.0.1] - 2025-11-20

### 🎉 Initial Release
- Official adoption of the GrowthBook Rust SDK.
- Basic feature flag evaluation.
- Remote feature fetching and caching. 
