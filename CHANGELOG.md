# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.0.5]

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