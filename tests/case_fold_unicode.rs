use growthbook_rust::client::{GrowthBookClientBuilder, GrowthBookClientTrait};
use growthbook_rust::model_public::GrowthBookAttribute;
use serde_json::json;

// Behavior consistency check -
// JS `String.prototype.toLowerCase()` does — the Unicode Default Case Algorithm,
// NOT an ASCII-only fold. Rust's `str::to_lowercase()` already follows that
// algorithm and This test is to ensure that behavior.
//
// Key non-ASCII cases (must NOT be treated as equal):
//   "İ".to_lowercase()      = "i\u{307}"  (i + combining dot) ≠ "i"
//   "STRAßE".to_lowercase() = "straße"                        ≠ "strasse"
// Simple 1:1 folds (must be equal):
//   "Σ".to_lowercase() = "σ";  "А".to_lowercase() = "а" (Cyrillic)
#[tokio::test]
async fn case_insensitive_fold_is_unicode_aware_like_js() {
    let features_json = json!({
        "fold-dotted-i": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$ini": ["İ"] } }, "force": true }]
        },
        "fold-sharp-s": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$ini": ["STRAßE"] } }, "force": true }]
        },
        "fold-sigma": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$ini": ["Σ"] } }, "force": true }]
        },
        "fold-cyrillic": {
            "defaultValue": false,
            "rules": [{ "condition": { "x": { "$ini": ["А"] } }, "force": true }]
        }
    });

    let client = GrowthBookClientBuilder::new().features_json(features_json).unwrap().build().await.expect("Failed to build client");

    let attr = |value: &str| GrowthBookAttribute::from(json!({ "x": value })).unwrap();

    // Non-ASCII folds that JS does NOT collapse → must not match.
    assert!(!client.is_on("fold-dotted-i", Some(attr("i"))), "İ must not fold to i");
    assert!(!client.is_on("fold-sharp-s", Some(attr("STRASSE"))), "ß must not expand to ss");

    // Simple Unicode 1:1 folds → must match.
    assert!(client.is_on("fold-sigma", Some(attr("σ"))), "Σ must fold to σ");
    assert!(client.is_on("fold-cyrillic", Some(attr("а"))), "Cyrillic А must fold to а");
}
