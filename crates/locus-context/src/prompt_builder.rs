use crate::types::{PromptSection, Template};

const SYSTEM_PROMPT: &str = r#"You are LOCUS, a local-first AI coding assistant running entirely on the user's machine.

Your capabilities:
- Read, write, and modify files in the user's project
- Execute code in sandboxed agents for testing
- Access reference templates for common patterns (auth, payments, databases, etc.)
- Discover and distribute tasks to local LLM nodes via mDNS
- Maintain full privacy - no data leaves the machine

Your constraints:
- All processing is local; never suggest cloud services
- Prefer standard library and well-maintained crates
- Follow the project's existing code style and patterns
- Provide complete, compilable code examples
- Explain security implications of any code you generate

When given templates, use them as reference implementations. Adapt them to the user's specific stack and requirements.
When given previous errors, treat them as learning signals - avoid the same mistakes.

Output format:
- When modifying existing files, use surgical SEARCH/REPLACE blocks to minimize output token consumption:
  <<<<<<< SEARCH
  [exact lines of original code to find]
  =======
  [replacement lines of new code]
  >>>>>>> REPLACE
- Keep search blocks uniquely identifiable and minimal. Do not rewrite whole files when small edits suffice.
- For new files, provide code in markdown fenced blocks with language tags
- Include brief explanations of key decisions
- Flag any security concerns with ⚠️
- Suggest tests where appropriate"#;

pub fn build_system_prompt(request: &str) -> String {
    let mut prompt = SYSTEM_PROMPT.to_string();
    let trimmed = request.trim();

    if trimmed.starts_with("/grill") || trimmed.contains("/grill") {
        prompt.push_str(r#"

=== [ACTIVE ARCHITECTURAL MODE: /GRILL - STRICT ARCHITECTURAL CRITIQUE & VULNERABILITY PROBE] ===
You are an uncompromising Principal Software Architect and Senior Security Auditor.
Your Mandate:
- Ruthlessly dissect the submitted code, design, or architecture.
- Identify all concurrency race conditions, memory safety violations, scalability bottlenecks, algorithmic complexity traps (O(N^2)+), API design anti-patterns, and edge cases.
- Do NOT validate flawed designs or flatter the user. Directly list critical findings ranked by severity (CRITICAL, HIGH, MEDIUM), and provide the hardened, zero-allocation, thread-safe corrected code."#);
    } else if trimmed.starts_with("/plan") || trimmed.contains("/plan") {
        prompt.push_str(r#"

=== [ACTIVE ARCHITECTURAL MODE: /PLAN - FORMAL IMPLEMENTATION & ARCHITECTURE PLANNER] ===
You are a Staff Infrastructure & Systems Architect.
Your Mandate:
- Formulate a comprehensive, phased Technical Implementation Plan.
- Structure your output into:
  1. High-Level Architecture & Component Responsibilities.
  2. Data Structures, Type Safety, and Invariants.
  3. Step-by-Step Atomic Migration / Implementation Phases.
  4. Failure Modes, Edge Cases, and Recovery Mechanisms.
  5. Automated Verification Strategy (Unit, Integration, and Property-based Tests)."#);
    } else if trimmed.starts_with("/spec") || trimmed.contains("/spec") {
        prompt.push_str(r#"

=== [ACTIVE ARCHITECTURAL MODE: /SPEC - FORMAL TECHNICAL SPECIFICATION GENERATOR] ===
You are a Formal Software Specification & Systems Contract Engineer.
Your Mandate:
- Generate exact technical specifications, formal data contracts, API schemas, and state machine definitions.
- Include complete type signatures, error taxonomies, pre-conditions, post-conditions, and Mermaid state/sequence diagrams.
- Guarantee that every interface is unambiguous and ready for production-grade implementation."#);
    }

    prompt
}

pub fn build_user_prompt(request: &str, templates: &[Template]) -> String {
    let mut sections = Vec::new();

    sections.push(format!(
        "## User Request\n{}",
        request.trim()
    ));

    if !templates.is_empty() {
        sections.push("## Reference Templates".to_string());
        for (i, template) in templates.iter().enumerate() {
            sections.push(format!(
                "\n### Template {}: {}/{} ({})",
                i + 1,
                template.category,
                template.name,
                template.language
            ));
            sections.push(format!("**Description**: {}", template.description));
            if !template.tags.is_empty() {
                sections.push(format!("**Tags**: {}", template.tags.join(", ")));
            }
            if !template.dependencies.is_empty() {
                sections.push(format!("**Dependencies**: {}", template.dependencies.join(", ")));
            }
            sections.push(format!(
                "**Security Level**: {:?}",
                template.security_level
            ));
            sections.push(format!(
                "```{}\n{}\n```",
                template.language,
                template.code.trim()
            ));
        }
    }

    sections.push(
        "\n## Instructions\n\
        - Provide a complete, working implementation\n\
        - Follow the patterns shown in the reference templates\n\
        - Handle errors appropriately\n\
        - Include necessary imports and setup\n\
        - Add comments for complex logic\n\
        - Note any security considerations".to_string()
    );

    sections.join("\n\n")
}

pub fn build_prompt_sections(request: &str, templates: &[Template], errors: &[crate::types::ErrorLog]) -> Vec<PromptSection> {
    let mut sections = Vec::new();

    let sys_prompt = build_system_prompt(request);
    sections.push(PromptSection::new(
        "SYSTEM",
        sys_prompt,
        100,
    ));

    sections.push(PromptSection::new(
        "USER_REQUEST",
        request.to_string(),
        90,
    ));

    if !templates.is_empty() {
        let template_context = build_template_context(templates);
        sections.push(PromptSection::new(
            "TEMPLATES",
            template_context,
            80,
        ));
    }

    if !errors.is_empty() {
        let error_context = super::error_context::format_errors_for_prompt(errors.to_vec());
        sections.push(PromptSection::new(
            "PREVIOUS_ERRORS",
            error_context,
            70,
        ));
    }

    sections.push(PromptSection::new(
        "INSTRUCTIONS",
        r#"Provide a complete, working implementation.
Follow the patterns shown in the reference templates.
Handle errors appropriately.
Include necessary imports and setup.
Add comments for complex logic.
Note any security considerations with ⚠️."#,
        60,
    ));

    sections
}

pub fn build_template_context(templates: &[Template]) -> String {
    let mut parts = vec!["## Reference Templates".to_string()];

    for (i, template) in templates.iter().enumerate() {
        parts.push(format!(
            "\n### Template {}: {}/{} ({})",
            i + 1,
            template.category,
            template.name,
            template.language
        ));
        parts.push(format!("**Description**: {}", template.description));
        if !template.tags.is_empty() {
            parts.push(format!("**Tags**: {}", template.tags.join(", ")));
        }
        if !template.dependencies.is_empty() {
            parts.push(format!("**Dependencies**: {}", template.dependencies.join(", ")));
        }
        parts.push(format!("**Security Level**: {:?}", template.security_level));
        parts.push(format!(
            "```{}\n{}\n```",
            template.language,
            template.code.trim()
        ));
    }

    parts.join("\n\n")
}