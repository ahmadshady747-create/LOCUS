//! Integration test suite for SafeTextInjector.

use locus_core::{InjectionReport, SafeTextInjector};

#[test]
fn test_safe_text_injector_execution() {
    let sample_text = "pub fn locus_injected_code() -> bool { true }";
    let report = SafeTextInjector::inject_text(sample_text, false);

    assert_eq!(report.bytes_injected, sample_text.len());
    assert!(report.elapsed_ms >= 0.0);
}

#[test]
fn test_safe_text_injector_multiline_and_unicode() {
    let arabic_markdown = "### تقرير التحقق الرياضي\n- **الحالة:** ناجح 100%\n- **الوقت:** <2.4ms";
    let report = SafeTextInjector::inject_text(arabic_markdown, false);

    assert_eq!(report.bytes_injected, arabic_markdown.len());
    assert!(report.elapsed_ms >= 0.0);
}

#[test]
fn test_injection_report_serialization() {
    let report = InjectionReport {
        bytes_injected: 1024,
        elapsed_ms: 1.85,
        clipboard_restored: true,
    };

    let json = serde_json::to_string(&report).expect("Failed to serialize InjectionReport");
    assert!(json.contains("\"bytes_injected\":1024"));
    assert!(json.contains("\"clipboard_restored\":true"));

    let deserialized: InjectionReport = serde_json::from_str(&json).expect("Failed to deserialize InjectionReport");
    assert_eq!(deserialized, report);
}
