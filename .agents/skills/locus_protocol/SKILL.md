---
name: locus_protocol
description: >-
  Architectural Gatekeeper enforcing the mandatory LOCUS Engine Verification & Synthesis Protocol.
  Triggers on any request to write, modify, refactor, or delete code across Rust, TypeScript, Python, or WGSL,
  or any architectural change involving AST syntax, memory allocation, or cross-module dependencies.
  Enforces pre-generation contract synthesis, token-efficient AST slicing, atomic verified patching,
  zero-panic invariant checking (32 passes), dynamic proof verification, and multi-file ACID workspace transactions.
---

# LOCUS Architect Skill (The Gatekeeper)

Autonomous protocol enforcing 100% deterministic safety, invariant compliance, and zero speculative generation across all codebases and languages (Rust, TypeScript, Python, WGSL, TSX, Svelte, Astro, Vue).

## 1. Trigger Conditions
Activate this skill unconditionally whenever:
- Writing, modifying, refactoring, or deleting code in any programming language (`Rust`, `TypeScript`, `Python`, `WGSL`, `TSX`, `JavaScript`).
- Generating structural architectures, data structures, or multi-file modules.
- Executing symbol modifications or refactoring shared dependencies across crates/packages.
- Performing formal AST invariant validation or security audits.

## 2. Hard Constraints & Prohibitions (Zero Tolerance)
- **NO Raw Unverified Edits:** NEVER use standard text replacement tools or write files directly to disk without routing through LOCUS verification.
- **NO Speculative Generation:** NEVER output boilerplate or multi-file implementations without synthesizing an invariant contract first.
- **Tool Precedence:** Tools provided by the `locus` MCP server (`synthesize_contract`, `prepare_context`, `verified_patch`, `check_safety`, `verify_contract`, `verify_proof`, `begin_tx`, `stage_tx`, `commit_tx`, `rollback_tx`) take absolute precedence over built-in editing tools.
- **Zero Unsafe:** Maintain 100% pure safe Rust standards (`#![forbid(unsafe_code)]`).

## 3. Mandatory Step-by-Step Execution Lifecycle

### Phase 1: Pre-Generation (Intent Synthesis & Blast Radius)
1. **Contract Synthesis:** Call `locus.synthesize_contract` with the developer's intent, language, and target path to establish the deterministic type scaffolding and invariant checklist.
2. **Context Intake:** Call `locus.prepare_context` or `locus.extract_intent_slice` to ingest token-efficient AST context (<0.25ms) instead of dumping entire raw files into the prompt.
3. **Blast Radius Analysis:** If modifying or refactoring shared symbols, call `locus.get_blast_radius` and `locus.find_references` to assess all downstream callers and breaking change risks.

### Phase 2: Atomic Generation & Surgical Patching
1. **Targeted Modifications:** Use `locus.verified_patch` or `locus.patch_symbol` to atomically validate in-memory AST safety prior to disk commit.
2. **Multi-File Workspace Operations:** When touching multiple files, wrap the changes in an ACID transaction:
   - Call `locus.begin_tx` to open the transaction.
   - Call `locus.stage_tx` for each modified file into the in-memory shadow buffer.
   - Run dry-run verification.
   - Call `locus.commit_tx` only if all files pass 0 AST violations; otherwise, invoke `locus.rollback_tx`.

### Phase 3: Post-Generation Invariant Verification & Proof
1. **Safety Scan:** Call `locus.check_safety` to ensure 0 AST violations across all 32 invariant rules (e.g., zero unguarded divisions, zero unhandled locks across await, zero client-side secret leaks, zero unbalanced JSX tags, zero catastrophic ReDoS).
2. **Contract Verification:** Call `locus.verify_contract` on the generated code to prove contract fidelity.
3. **Formal Dynamic Proof:** Call `locus.verify_proof` to evaluate AST proof assertions and mint a cryptographic FIPS 180-4 SHA-256 certificate (`LOCUS-PROOF-v2`).
4. **Self-Correction Requirement:** If any LOCUS tool reports an invariant violation, you MUST self-correct immediately and re-verify before presenting results to the user.

## 4. Execution Proof Requirement
Every code response MUST cite the exact LOCUS verification status, latency, and confirmation of 0 AST invariant violations.
