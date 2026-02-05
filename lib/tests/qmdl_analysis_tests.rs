//! Integration tests using QMDL fixture files.
//!
//! These tests verify the analysis pipeline produces expected results
//! when processing real or crafted QMDL captures.
//!
//! See `tests/fixtures/README.md` for information on adding fixtures.

use std::pin::pin;

use rayhunter::analysis::analyzer::{AnalyzerConfig, EventType, Harness};
use rayhunter::qmdl::QmdlReader;
use std::path::PathBuf;
use tokio::fs::File;

/// Get the path to a test fixture file
fn fixture_path(name: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests");
    path.push("fixtures");
    path.push(name);
    path
}

/// Analyze a QMDL fixture file and return the maximum event severity seen
async fn analyze_fixture(name: &str, config: AnalyzerConfig) -> Option<EventType> {
    let path = fixture_path(name);
    if !path.exists() {
        // Skip test if fixture doesn't exist yet
        return None;
    }

    let file = File::open(&path).await.expect("failed to open fixture");
    let file_size = file.metadata().await.expect("failed to get metadata").len();
    let mut reader = QmdlReader::new(file, Some(file_size as usize));
    let mut harness = Harness::new_with_config(&config);
    let mut max_severity: Option<EventType> = None;

    use futures::{StreamExt, TryStreamExt};
    let mut stream = pin!(reader.as_stream().into_stream());
    while let Some(result) = stream.next().await {
        let container = result.expect("failed to read container");
        let rows = harness.analyze_qmdl_messages(container);
        for row in rows {
            for event in row.events.into_iter().flatten() {
                match &max_severity {
                    None => max_severity = Some(event.event_type),
                    Some(current) if event.event_type > *current => {
                        max_severity = Some(event.event_type)
                    }
                    _ => {}
                }
            }
        }
    }

    max_severity
}

/// Test that clean baseline capture produces no warnings (false positive check)
#[tokio::test]
async fn test_clean_baseline_no_false_positives() {
    let result = analyze_fixture("clean_baseline.qmdl", AnalyzerConfig::default()).await;

    // If fixture exists, verify no warnings above Informational
    if let Some(max_event) = result {
        assert!(
            max_event <= EventType::Informational,
            "Clean baseline should not trigger warnings, but got {:?}",
            max_event
        );
    }
    // If fixture doesn't exist, test is skipped (returns None)
}

// Future tests to add when fixtures are available:
//
// #[tokio::test]
// async fn test_null_cipher_detection() {
//     let result = analyze_fixture("null_cipher_attack.qmdl", AnalyzerConfig::default()).await;
//     assert!(matches!(result, Some(EventType::High)));
// }
//
// #[tokio::test]
// async fn test_imsi_request_detection() {
//     let result = analyze_fixture("imsi_request.qmdl", AnalyzerConfig::default()).await;
//     assert!(matches!(result, Some(EventType::High)));
// }
