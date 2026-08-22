# LOCUS Workspace Guidelines & Architectural Protocol

You are strictly bound by the **LOCUS Engine Verification & Synthesis Protocol**. Under NO circumstances are you allowed to bypass, skip, or ignore `locus` MCP tools when analyzing, planning, generating, or modifying code across ANY workspace or language (`Rust`, `TypeScript`, `JavaScript`, `TSX/JSX`, `Svelte`, `Astro`, `Vue`, `Python`).

### 1. Hard Constraints & Prohibitions (Zero Tolerance)
- **NO Raw Unverified Edits:** You MUST NEVER edit or write code directly without passing through LOCUS validation.
- **NO Speculative Generation:** You MUST NOT generate structural boilerplate or multi-file logic without synthesizing a contract first.
- **Tool Selection Precedence:** LOCUS tools take absolute precedence over standard built-in file editing and inspection tools.
- **Zero Unsafe:** Maintain 100% safe Rust standards across all core subsystems.

### 2. Mandatory Step-by-Step Execution Lifecycle

#### Phase 1: Intent Synthesis & Context Intake (Pre-Generation)
Before generating or modifying any module, you MUST:
1. Call `locus.synthesize_contract` with the developer's intent and target language to establish the type scaffolding and deterministic invariant checklist.
2. Call `locus.prepare_context` or `locus.extract_intent_slice` to ingest token-efficient AST context instead of reading entire raw source files.
3. If modifying existing shared symbols, call `locus.get_blast_radius` to identify all downstream callers and assess breaking change risks.

#### Phase 2: Atomic Generation & Surgical Patching
When applying changes or creating implementations:
1. For targeted function/symbol modifications: Execute `locus.verified_patch` or `locus.patch_symbol`.
2. Ensure the generated implementation strictly adheres to the synthesized type contracts and error variants.

#### Phase 3: Bidirectional Safety & Invariant Verification (Post-Generation)
Before presenting your final response to the user:
1. Call `locus.verify_contract` on the generated code to prove contract fidelity.
2. Call `locus.check_safety` to ensure 0 AST invariant violations (zero unhandled hook calls, zero client-side secret leaks, zero unbalanced JSX tags, zero unhandled async locks).
3. If LOCUS reports any violation, you MUST self-correct and re-verify immediately before replying.

### 3. Execution Proof Requirement
In every code generation or refactoring task, your response MUST reference the exact LOCUS verification status or tool execution output confirming that AST invariants and contracts passed.
