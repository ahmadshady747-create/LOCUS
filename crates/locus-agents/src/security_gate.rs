//! Zero-Shot Micro-SAST Deterministic Security Gate.
//!
//! Provides ultra-fast (<2ms), deterministic static security analysis and secret scanning
//! directly in-memory before file modifications or code generations are committed.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecuritySeverity {
    Info,
    Warning,
    Critical,
    Blocker,
}

impl SecuritySeverity {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Critical => "CRITICAL",
            Self::Blocker => "BLOCKER",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityViolationCategory {
    SecretLeak,
    SqlInjection,
    CommandInjection,
    PathTraversal,
    UndocumentedUnsafe,
    HardcodedCredentials,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityViolation {
    pub id: String,
    pub category: SecurityViolationCategory,
    pub severity: SecuritySeverity,
    pub title: String,
    pub description: String,
    pub line_number: Option<usize>,
    pub snippet: String,
    pub remediation_advice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanResult {
    pub is_safe: bool,
    pub violations: Vec<SecurityViolation>,
    pub entropy_alerts: usize,
    pub scan_duration_micros: u64,
    pub summary: String,
}

pub struct SecurityGate;

impl SecurityGate {
    /// Scans a code snippet for secret leaks, structural vulnerabilities, and undocumented unsafe blocks.
    pub fn validate_snippet(code: &str, language: Option<&str>) -> SecurityScanResult {
        let start = Instant::now();
        let mut violations = Vec::new();
        let mut entropy_alerts = 0;

        let lines: Vec<&str> = code.lines().collect();

        // 1. Scan for Secrets & API Keys via Patterns & Shannon Entropy
        let re_secret_patterns = [
            (
                "AWS Access Key",
                Regex::new(r#"(?i)(?:aws_access_key_id|aws_secret_access_key|AKIA[0-9A-Z]{16})"#).unwrap(),
            ),
            (
                "GitHub Token",
                Regex::new(r#"(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,}"#).unwrap(),
            ),
            (
                "OpenAI / Anthropic API Key",
                Regex::new(r#"(?:sk-[A-Za-z0-9]{32,}|sk-ant-[A-Za-z0-9]{32,})"#).unwrap(),
            ),
            (
                "Google Gemini / Cloud API Key",
                Regex::new(r#"AIzaSy[A-Za-z0-9_-]{33}"#).unwrap(),
            ),
            (
                "Private Key Header",
                Regex::new(r#"-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----"#).unwrap(),
            ),
            (
                "Bearer Authorization Header",
                Regex::new(r#"(?i)bearer\s+[a-zA-Z0-9_\-\.]{30,}"#).unwrap(),
            ),
        ];

        let re_string_literals = Regex::new(r#"(?:"([^"\\]*)"|'([^'\\]*)')"#).unwrap();

        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();

            // Ignore comments
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                continue;
            }

            // Check known secret regexes
            for (title, re) in &re_secret_patterns {
                if re.is_match(trimmed) {
                    violations.push(SecurityViolation {
                        id: format!("SEC-KEY-{}", violations.len() + 1),
                        category: SecurityViolationCategory::SecretLeak,
                        severity: SecuritySeverity::Blocker,
                        title: format!("Hardcoded {}", title),
                        description: format!("Detected raw {} embedded directly in code.", title),
                        line_number: Some(line_num),
                        snippet: trimmed.to_string(),
                        remediation_advice: "Extract secret into environment variables (.env) or KeyVault.".to_string(),
                    });
                }
            }

            // Shannon Entropy check on long string literals
            for caps in re_string_literals.captures_iter(trimmed) {
                let literal = caps.get(1).or_else(|| caps.get(2)).map(|m| m.as_str()).unwrap_or("");
                if literal.len() >= 24 && !literal.contains(' ') {
                    let entropy = Self::calculate_shannon_entropy(literal);
                    if entropy > 4.5 {
                        entropy_alerts += 1;
                        if !violations.iter().any(|v| v.line_number == Some(line_num)) {
                            violations.push(SecurityViolation {
                                id: format!("SEC-ENTROPY-{}", violations.len() + 1),
                                category: SecurityViolationCategory::SecretLeak,
                                severity: SecuritySeverity::Critical,
                                title: "High-Entropy Secret String".to_string(),
                                description: format!("String literal has high Shannon entropy ({:.2}), indicating a possible password, token, or cryptographic secret.", entropy),
                                line_number: Some(line_num),
                                snippet: trimmed.to_string(),
                                remediation_advice: "Move high-entropy credentials out of source code into environment variables.".to_string(),
                            });
                        }
                    }
                }
            }

            // 2. Scan for SQL Injection (Unparameterized raw formatting)
            let re_sql = Regex::new(r#"(?i)(?:format!\s*\(\s*"(?:SELECT|INSERT|UPDATE|DELETE)\b.*\{\}|f"(?:SELECT|INSERT|UPDATE|DELETE)\b.*\{)"#).unwrap();
            if re_sql.is_match(trimmed) {
                violations.push(SecurityViolation {
                    id: format!("SEC-SQL-{}", violations.len() + 1),
                    category: SecurityViolationCategory::SqlInjection,
                    severity: SecuritySeverity::Blocker,
                    title: "SQL Injection Risk".to_string(),
                    description: "Unparameterized raw string formatting detected inside SQL query.".to_string(),
                    line_number: Some(line_num),
                    snippet: trimmed.to_string(),
                    remediation_advice: "Use parameterized queries ($1, ? or query bindings) instead of direct string interpolation.".to_string(),
                });
            }

            // 3. Scan for Command Injection (Raw shell execution on variables)
            let re_cmd = Regex::new(r#"(?i)(?:os\.system\s*\(|exec\s*\(\s*f["']|eval\s*\(|Command::new\s*\(\s*["'](?:sh|bash|cmd|powershell)["']\s*\)\s*\.arg\s*\(\s*["']-[cC]["']\s*\)\s*\.arg\s*\(\s*format!)"#).unwrap();
            if re_cmd.is_match(trimmed) {
                violations.push(SecurityViolation {
                    id: format!("SEC-CMD-{}", violations.len() + 1),
                    category: SecurityViolationCategory::CommandInjection,
                    severity: SecuritySeverity::Blocker,
                    title: "Command Injection Risk".to_string(),
                    description: "Raw shell string execution on dynamically formatted arguments.".to_string(),
                    line_number: Some(line_num),
                    snippet: trimmed.to_string(),
                    remediation_advice: "Pass arguments as individual array parameters to Command::new rather than a single interpolated shell string.".to_string(),
                });
            }

            // 4. Scan for Path Traversal (Raw "../" concatenation)
            let re_path = Regex::new(r#"(?i)(?:\.\./\.\.|Path::new\s*\(\s*format!.*(?:\.\./|\.\.\\))"#).unwrap();
            if re_path.is_match(trimmed) {
                violations.push(SecurityViolation {
                    id: format!("SEC-PATH-{}", violations.len() + 1),
                    category: SecurityViolationCategory::PathTraversal,
                    severity: SecuritySeverity::Critical,
                    title: "Path Traversal Risk".to_string(),
                    description: "Arbitrary relative path traversal ('../') without canonicalization verification.".to_string(),
                    line_number: Some(line_num),
                    snippet: trimmed.to_string(),
                    remediation_advice: "Canonicalize path and verify that it remains within the allowed root workspace directory.".to_string(),
                });
            }

            // 5. Scan for Undocumented Unsafe in Rust
            let lang_clean = language.unwrap_or("").to_lowercase();
            if (lang_clean == "rust" || lang_clean == "rs") && trimmed.starts_with("unsafe {") {
                let prev_line = if idx > 0 { lines[idx - 1].trim() } else { "" };
                if !prev_line.to_uppercase().contains("SAFETY:") {
                    violations.push(SecurityViolation {
                        id: format!("SEC-UNSAFE-{}", violations.len() + 1),
                        category: SecurityViolationCategory::UndocumentedUnsafe,
                        severity: SecuritySeverity::Warning,
                        title: "Undocumented Unsafe Block".to_string(),
                        description: "Rust `unsafe` block declared without a preceding `// SAFETY:` explanatory contract comment.".to_string(),
                        line_number: Some(line_num),
                        snippet: trimmed.to_string(),
                        remediation_advice: "Add a `// SAFETY: <explanation of invariants>` comment right above the unsafe block.".to_string(),
                    });
                }
            }
        }

        let is_safe = violations.iter().all(|v| v.severity != SecuritySeverity::Blocker && v.severity != SecuritySeverity::Critical);
        let scan_duration_micros = start.elapsed().as_micros() as u64;

        let summary = if is_safe {
            "Zero vulnerabilities detected. Code passes all deterministic SAST security gates.".to_string()
        } else {
            format!("Security Gate Alert: Found {} critical/blocker violation(s).", violations.len())
        };

        SecurityScanResult {
            is_safe,
            violations,
            entropy_alerts,
            scan_duration_micros,
            summary,
        }
    }

    /// Validates a unified diff snippet before application.
    pub fn validate_diff(diff: &str) -> SecurityScanResult {
        // Extract added lines from diff
        let mut added_lines = Vec::new();
        for line in diff.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                added_lines.push(&line[1..]);
            }
        }
        let added_code = added_lines.join("\n");
        Self::validate_snippet(&added_code, None)
    }

    /// Computes Shannon entropy: H(X) = -sum(P(x) * log2(P(x)))
    pub fn calculate_shannon_entropy(s: &str) -> f64 {
        if s.is_empty() {
            return 0.0;
        }

        let mut counts = HashMap::new();
        for ch in s.chars() {
            *counts.entry(ch).or_insert(0) += 1;
        }

        let len = s.len() as f64;
        let mut entropy = 0.0;

        for &count in counts.values() {
            let p = count as f64 / len;
            entropy -= p * p.log2();
        }

        entropy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shannon_entropy_calculation() {
        let low_entropy = "aaaaaaaaaaaaaaaaaaaa";
        assert!(SecurityGate::calculate_shannon_entropy(low_entropy) < 1.0);

        let high_entropy = "aK9#zL!0pQ2$mX8&vR4*jB6@";
        assert!(SecurityGate::calculate_shannon_entropy(high_entropy) > 4.2);
    }

    #[test]
    fn test_secret_detection_openai_key() {
        let snippet = "const apiKey = 'sk-abcdef12345678901234567890123456';";
        let res = SecurityGate::validate_snippet(snippet, Some("typescript"));
        assert!(!res.is_safe);
        assert!(res.violations.iter().any(|v| v.category == SecurityViolationCategory::SecretLeak));
    }

    #[test]
    fn test_sql_injection_detection() {
        let snippet = r#"let query = format!("SELECT * FROM users WHERE id = {}", user_id);"#;
        let res = SecurityGate::validate_snippet(snippet, Some("rust"));
        assert!(!res.is_safe);
        assert!(res.violations.iter().any(|v| v.category == SecurityViolationCategory::SqlInjection));
    }

    #[test]
    fn test_command_injection_detection() {
        let snippet = r#"os.system(f"rm -rf {user_directory}")"#;
        let res = SecurityGate::validate_snippet(snippet, Some("python"));
        assert!(!res.is_safe);
        assert!(res.violations.iter().any(|v| v.category == SecurityViolationCategory::CommandInjection));
    }

    #[test]
    fn test_undocumented_unsafe_detection_rust() {
        let snippet = "unsafe {\n    *ptr = 42;\n}";
        let res = SecurityGate::validate_snippet(snippet, Some("rust"));
        assert!(res.violations.iter().any(|v| v.category == SecurityViolationCategory::UndocumentedUnsafe));

        let safe_documented = "// SAFETY: ptr is guaranteed to be non-null and aligned\nunsafe {\n    *ptr = 42;\n}";
        let res2 = SecurityGate::validate_snippet(safe_documented, Some("rust"));
        assert!(!res2.violations.iter().any(|v| v.category == SecurityViolationCategory::UndocumentedUnsafe));
    }
}
