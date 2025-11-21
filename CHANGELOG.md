# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2025-01-27

### 🧪 Testing Improvements
- **Enhanced Testability**: Moved `FeatureResult`, `Experiment`, and `ExperimentResult` from `model_private` to `model_public`
  - Enables creation of mock `FeatureResult` instances for testing
  - Allows custom implementations of `GrowthBookClientTrait` for unit tests
  - Provides public constructors for easier test setup
  - Supports mocking feature flag responses in integration tests

### 🔧 API Improvements
- **Public Constructors**: Added `FeatureResult::new()` constructor for custom instances
  - Enables creation of test-specific `FeatureResult` instances
  - Maintains all existing constructors (`force()`, `experiment()`, etc.)
  - No breaking changes to existing API

### 🏗️ Architecture
- **Module Reorganization**: Removed `model_private.rs` module
  - All structures now properly accessible for testing
  - Cleaner module structure with better separation of concerns
  - Maintains backward compatibility

### 📝 Documentation
- **Updated Examples**: Enhanced client example with `FeatureResult` usage demonstration
  - Shows how to create custom `FeatureResult` instances
  - Demonstrates test-friendly API design

---

## [1.0.0] - 2025-01-27

### 🚀 Features
- **Added `GrowthBookClientTrait`**: Introduced a new trait interface for the GrowthBook client
  - Provides abstraction layer for feature flag operations
  - Enables trait object usage with `Box<dyn GrowthBookClientTrait>` and `Arc<dyn GrowthBookClientTrait>`
  - Supports `Debug + Send + Sync` bounds for thread-safe operations
  - Methods: `is_on()`, `is_off()`, `feature_result()`, `total_features()`

### 🔧 Improvements
- **Enhanced API Design**: Better separation of concerns with trait-based interface
- **Thread Safety**: Explicit `Send + Sync` bounds for async/multithread usage
- **Debug Support**: `Debug` bound for logging and debugging capabilities
- **Future Extensibility**: Trait enables multiple implementations and easier testing

### 🧪 Testing
- **Updated Test Suite**: All tests now use the trait interface
- **Maintained Coverage**: 67 tests passing with no regressions
- **Improved Testability**: Trait enables easier mocking and testing

---

## [0.1.2] - 2025-06-16

### 🔧 Improvements
- **Version Bump**: Updated from 0.1.1 to 0.1.2
- **Dependency Updates**: Minor dependency version adjustments

---

## [0.1.1] - 2025-06-16

### 🐛 Bug Fixes
- **HTTP Connection Management**: Fixed pool idle timeout configuration
  - Set `pool_idle_timeout` to `None` to prevent connection issues
  - Improved connection handling for long-running applications

### 🔧 Improvements
- **Keep-Alive Configuration**: Set keep-alive off as default
  - Prevents connection pooling issues in certain environments
  - More predictable connection behavior

---

## [0.0.3] - 2025-06-16

### 🔧 Improvements
- **Dependency Updates**: Bumped reqwest-tracing and middleware versions
  - Improved compatibility with other packages
  - Better integration with tracing infrastructure

---

## Breaking Changes

### ⚠️ Version 1.1.0
**No breaking changes** - All changes are fully backward compatible:

- ✅ **Existing code continues to work unchanged**
- ✅ **All public APIs remain the same**
- ✅ **No changes to method signatures**
- ✅ **No changes to return types**
- ✅ **No changes to error handling**
- ✅ **FeatureResult constructors remain the same**

### Migration Guide
No migration required. The changes are additive and improve testability:

```rust
// Before (still works):
let result = client.feature_result("feature", None);
assert!(result.on);

// After (same behavior, plus new capabilities):
let result = client.feature_result("feature", None);
assert!(result.on);

// New capability - create custom FeatureResult for testing:
let mock_result = FeatureResult::new(
    serde_json::json!("test-value"),
    true,
    "test".to_string()
);
```

### ⚠️ Version 1.0.0
**No breaking changes** - The addition of `GrowthBookClientTrait` is fully backward compatible:

- ✅ **Existing code continues to work unchanged**
- ✅ **All public APIs remain the same**
- ✅ **No changes to method signatures**
- ✅ **No changes to return types**
- ✅ **No changes to error handling**

### Migration Guide
No migration required. The trait is additive and doesn't affect existing usage:

```rust
// Before (still works):
let client = GrowthBookClient::new(url, key, None, None).await?;
let is_on = client.is_on("feature", None);

// After (same behavior):
let client = GrowthBookClient::new(url, key, None, None).await?;
let is_on = client.is_on("feature", None);

// New capability (optional):
let boxed_client: Box<dyn GrowthBookClientTrait> = Box::new(client);
let is_on = boxed_client.is_on("feature", None);
```

---

## Dependencies

### Updated Dependencies
- No breaking dependency changes in this release
- All existing dependencies remain compatible
- New trait functionality uses existing dependencies

### Minimum Supported Rust Version
- Rust 1.70+ (unchanged)

---

## Contributors

- @gabrielsartorato - Enhanced testability and GrowthBookClientTrait
- @carlos.marega - HTTP connection improvements
- @fernando.goncalves - Version management

---

## Notes

- This release maintains full backward compatibility
- The changes improve testability without breaking existing code
- All existing functionality remains unchanged
- Performance characteristics are identical to previous versions
- Enhanced support for unit testing and mocking scenarios 