---
name: locus-engine
description: >-
  High-throughput compound MCP pipelines, deterministic AST safety verification, surgical symbol patching,
  context compression, cross-file dependency graph resolver, and blast-radius impact analyzer powered by native locus-engine (v0.3.3).
  Activate this skill when generating, reviewing, patching, or analyzing code in Rust, TypeScript, TSX, JSX, Svelte, Astro, Vue, or Python.
---

# LOCUS Engine Skill Guide (v0.3.3)

Use this skill to leverage the native `locus` MCP server and CLI for sub-millisecond invariant safety verification, compound context preparation, blast-radius impact analysis, and verified atomic patching across Backend (Rust, Python) and Frontend (TSX, JSX, Svelte, Astro, Vue) ecosystems.

---

## ⚡ High-Throughput Compound Pipelines (PRIORITIZE THESE):

To eliminate LLM round-trip latency, ALWAYS prioritize these two compound atomic tools over individual micro-calls:

### 1. `prepare_context` (Consolidated Context Pipeline):
- **When to use:** **FIRST STEP BEFORE GENERATING CODE**. Extracts the file AST skeleton, intent context slice, blast radius, resolved symbol, and token savings in a single unified pass (<0.2ms).
- **Usage:** Call `prepare_context` with `{"target_file": "src/guard.rs", "symbol": "AstGuard", "budget": 1000}`.
- **Payload Returned:**
  - `file_skeleton`: Compact structural skeleton (>73% token reduction).
  - `sliced_context`: High-density AST slice containing only the target symbol and direct dependencies.
  - `blast_radius`: Downstream callers, impacted files, and breaking change risk score.
  - `resolved_symbol`: Signatures, origin byte-spans, and doc comments.

### 2. `verified_patch` (Consolidated Atomic Patching Pipeline):
- **When to use:** **PRIMARY TOOL FOR COMMITTING CODE EDITS**. Atomically executes:
  1. Pre-patch invariant safety verification on `new_code`.
  2. In-memory AST symbol replacement.
  3. Post-patch full-file integrity validation.
  4. Atomic disk write (if valid).
  - *If any safety invariant is violated or syntax errors exist, it aborts without touching disk and returns exact diagnostics.*
- **Usage:** Call `verified_patch` with `{"file_path": "src/auth.rs", "symbol": "login", "new_code": "pub async fn login(...) { ... }"}`.

---

## 🛠️ Specialized Granular Tools:

When fine-grained control is needed, the following individual tools are available:

1. `get_blast_radius`:
   - Calculates downstream breaking change risk, caller sites, and impacted file list before refactoring.
   - **Usage:** Call `get_blast_radius` with `{"symbol": "AstGuard", "path": "src/", "depth": 2}`.

2. `resolve_symbol`:
   - Resolves symbol origin file, byte coordinates, type signature, and doc-comments across module paths.
   - **Usage:** Call `resolve_symbol` with `{"symbol": "UserProfileCard", "from_file": "src/Dashboard.tsx", "target_path": "src/"}`.

3. `find_references`:
   - Locates all call sites, imports, and usages of a symbol across the entire workspace.
   - **Usage:** Call `find_references` with `{"symbol": "check_safety", "target_path": "src/"}`.

4. `synthesize_contract`:
   - Projects developer intent into strict type scaffolding and safety invariant checklists before implementation.
   - **Usage:** Call `synthesize_contract` with `{"intent": "...", "target_path": "src/auth.rs", "language": "rust|tsx|python"}`.

5. `extract_intent_slice`:
   - Extracts a minimal AST context slice containing only the target symbol and its direct dependencies.
   - **Usage:** Call `extract_intent_slice` with `{"symbol": "UserProfileCard", "code": "...", "depth": 2}`.

6. `verify_contract`:
   - Bidirectionally verifies generated code against agreed type contracts with zero safety violations.
   - **Usage:** Call `verify_contract` with `{"intent": "...", "generated_code": "...", "language": "rust|tsx|python"}`.

7. `check_safety`:
   - Deterministic 11-pass AST safety firewall (delimiters, JSX tags, rules of hooks, secret leaks, unwraps, deadlocks, ReDoS).
   - **Usage:** Call `check_safety` with `{"code": "..."}` or `{"path": "src/Component.tsx"}`.

8. `skeletonize`:
   - Extracts structural AST skeleton (>70-85% token reduction).
   - **Usage:** Call `skeletonize` with `{"code": "...", "language": "tsx"}`.

9. `patch_symbol`:
   - Surgically replaces a named AST symbol with new code.
   - **Usage:** Call `patch_symbol` with `{"source": "...", "symbol": "login", "new_code": "..."}`.

10. `index_graph`:
    - Indexes workspace into a cross-file SymbolGraph and reports architectural health (cycles & orphan exports).
    - **Usage:** Call `index_graph` with `{"path": "src/"}`.
