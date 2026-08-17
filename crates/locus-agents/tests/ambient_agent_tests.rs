//! Integration test suite for AmbientAgentEngine.

use locus_agents::AmbientAgentEngine;

#[tokio::test]
async fn test_ambient_agent_python_to_rust_translation() {
    let python_fn = r#"
def divide(a, b):
    return a / b
"#;

    let result = AmbientAgentEngine::execute_ambient_action("convert to rust", Some(python_fn))
        .await
        .expect("Failed to execute ambient action");

    assert!(result.generated_patch.is_some());
    let patch = result.generated_patch.unwrap();
    assert!(patch.contains("pub fn divide"));
    assert!(patch.contains("b != 0"));
    assert!(result.verification_passed, "Generated guarded Rust code must pass formal verification");
}

#[tokio::test]
async fn test_ambient_agent_arabic_command_translation() {
    let python_fn = r#"
def multiply(a, b):
    return a * b
"#;

    let result = AmbientAgentEngine::execute_ambient_action("حوّل هذه الدالة إلى Rust", Some(python_fn))
        .await
        .expect("Failed to execute ambient action");

    assert!(result.generated_patch.is_some());
    let patch = result.generated_patch.unwrap();
    assert!(patch.contains("pub fn multiply"));
    assert!(result.verification_passed);
}

#[tokio::test]
async fn test_ambient_agent_array_bounds_fix() {
    let unsafe_indexing = r#"
pub fn read_first(arr: &[i32]) -> i32 {
    arr[0]
}
"#;

    let result = AmbientAgentEngine::execute_ambient_action("أصلح فحص الحدود", Some(unsafe_indexing))
        .await
        .expect("Failed to execute ambient action");

    assert!(result.generated_patch.is_some());
    let patch = result.generated_patch.unwrap();
    assert!(patch.contains(".get(0)"));
    assert!(result.verification_passed, "Guarded index access must pass formal verification");
}
