use crate::types::ErrorLog;

pub fn format_errors_for_prompt(errors: Vec<ErrorLog>) -> String {
    if errors.is_empty() {
        return String::new();
    }

    let mut parts = vec!["## Previous Errors (Learn from these)".to_string()];

    for (i, error) in errors.iter().enumerate() {
        parts.push(format!(
            "\n### Error {} (Agent: {}, Time: {})",
            i + 1,
            error.agent_id,
            error.timestamp
        ));
        parts.push(format!("**Message**: {}", error.error_message));

        if let Some(ctx) = &error.context {
            parts.push(format!("**Context**: {}", ctx));
        }

        if let Some(trace) = &error.stack_trace {
            parts.push(format!("**Stack Trace**:\n```\n{}\n```", trace));
        }

        parts.push("**Action**: Avoid this pattern in your solution.".to_string());
    }

    parts.join("\n")
}

pub fn format_errors_compact(errors: &[ErrorLog]) -> String {
    if errors.is_empty() {
        return String::new();
    }

    errors
        .iter()
        .enumerate()
        .map(|(i, e)| {
            format!(
                "{}. [{}] {}",
                i + 1,
                e.agent_id,
                e.error_message.lines().next().unwrap_or(&e.error_message)
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn deduplicate_errors(errors: Vec<ErrorLog>) -> Vec<ErrorLog> {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    errors
        .into_iter()
        .filter(|e| {
            let key = format!("{}:{}", e.agent_id, e.error_message);
            seen.insert(key)
        })
        .collect()
}

pub fn filter_recent_errors(errors: Vec<ErrorLog>, max_age_hours: u64) -> Vec<ErrorLog> {
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_hours as i64);
    errors
        .into_iter()
        .filter(|e| {
            chrono::DateTime::parse_from_rfc3339(&e.timestamp)
                .map(|dt| dt.with_timezone(&chrono::Utc) > cutoff)
                .unwrap_or(true)
        })
        .collect()
}