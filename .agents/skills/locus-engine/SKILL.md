---
name: locus-engine
description: >-
  High-throughput compound MCP pipelines, deterministic 20-pass AST safety verification, multi-agent symbol leases,
  cross-file taint tracking, pure-Rust HNSW vector search, deterministic self-healing, multi-file ACID workspace transactions,
  surgical symbol patching, and blast-radius impact analyzer powered by native locus-engine (v1.5.0).
  Activate this skill when generating, reviewing, patching, or analyzing code in Rust, TypeScript, TSX, JSX, Svelte, Astro, Vue, or Python.
---

# LOCUS Engine Skill Guide (v1.5.0)

Use this skill to leverage the native `locus` MCP server and CLI for sub-millisecond invariant safety verification, compound context preparation, blast-radius impact analysis, multi-agent concurrency leases, cross-file taint analysis, in-memory HNSW search, and ACID workspace transactions across Backend (Rust, Python) and Frontend (TSX, JSX, Svelte, Astro, Vue) ecosystems.

---

## ⚡ High-Throughput Compound Pipelines (PRIORITIZE THESE):

To eliminate LLM round-trip latency, ALWAYS prioritize these compound atomic tools over individual micro-calls:

### 1. `prepare_context` (Consolidated Context Pipeline):
- **When to use:** **FIRST STEP BEFORE GENERATING CODE**. Extracts the file AST skeleton, intent context slice, blast radius, resolved symbol, and token savings in a single unified pass (<0.25ms).
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

## 🔒 Multi-Agent Swarm Governance & ACID Transactions (v1.5.0):

1. `acquire_symbol_lease`:
   - Locks a Fully Qualified Symbol Name (`FQN`) with a short-lived TTL to prevent multi-agent write collisions.
   - **Usage:** `{"fqn": "src/auth.rs::login", "agent_id": "agent_alpha", "ttl_ms": 60000}`.

2. `release_symbol_lease`:
   - Releases an active symbol lease held by an agent.
   - **Usage:** `{"lease_id": "<lease_id>", "agent_id": "agent_alpha"}`.

3. `renew_symbol_lease`:
   - Renews heartbeat extension on an active lease.
   - **Usage:** `{"lease_id": "<lease_id>", "agent_id": "agent_alpha", "extension_ms": 60000}`.

4. `begin_tx`, `stage_tx`, `commit_tx`, `rollback_tx`:
   - Multi-file ACID workspace transactions staging files in-memory and committing only if 100% of invariants pass.
   - **Usage:**
     - `begin_tx` -> `{"tx_id": "tx_refactor"}`
     - `stage_tx` -> `{"path": "src/models.rs", "content": "..."}`
     - `commit_tx` -> `{"dry_run": false}`
     - `rollback_tx` -> `{}`

---

## 🔍 In-Memory Semantic Search & Cross-File Taint Tracking (v1.5.0):

1. `hybrid_search`:
   - Sub-millisecond in-memory hybrid AST lexical + quantized HNSW vector search (<1ms, <12MB RAM).
   - **Usage:** `{"query": "authenticate user jwt", "path": "src/", "top_k": 5}`.

2. `trace_taint_flow`:
   - Traces unvalidated external inputs and unhandled `Option<T>` returns to sensitive sinks.
   - **Usage:** `{"file_path": "src/upload.ts", "symbol": "handleUpload"}`.

3. `auto_remediate`:
   - Deterministic AST self-healing rewriter (unclosed JSX, deep null optional chaining `?.`, conditional hook hoisting).
   - **Usage:** `{"code": "<div><p>Hello"}`.

---

## 🛠️ Specialized Granular Tools:

1. `check_safety`:
   - Deterministic 20-pass AST safety firewall (<0.05ms) returning exact byte-level counterexamples.
   - **Usage:** `{"code": "..."}` or `{"path": "src/Component.tsx"}`.

2. `synthesize_contract`:
   - Projects developer intent into strict type scaffolding and safety invariant checklists before implementation.
   - **Usage:** `{"intent": "...", "target_path": "src/auth.rs", "language": "rust|tsx|python"}`.

3. `verify_contract`:
   - Bidirectionally verifies generated code against agreed type contracts with zero safety violations.
   - **Usage:** `{"intent": "...", "generated_code": "...", "language": "rust|tsx|python"}`.

4. `get_blast_radius`:
   - Calculates downstream breaking change risk, caller sites, and impacted file list before refactoring.
   - **Usage:** `{"symbol": "AstGuard", "path": "src/", "depth": 2}`.

5. `resolve_symbol`:
   - Resolves symbol origin file, byte coordinates, type signature, and doc-comments across module paths.
   - **Usage:** `{"symbol": "UserProfileCard", "from_file": "src/Dashboard.tsx", "target_path": "src/"}`.

6. `find_references`:
   - Locates all call sites, imports, and usages of a symbol across the entire workspace.
   - **Usage:** `{"symbol": "check_safety", "target_path": "src/"}`.

7. `extract_intent_slice`:
   - Extracts a minimal AST context slice containing only the target symbol and its direct dependencies.
   - **Usage:** `{"symbol": "UserProfileCard", "code": "...", "depth": 2}`.

8. `skeletonize`:
   - Extracts structural AST skeleton (>70-85% token reduction).
   - **Usage:** `{"code": "...", "language": "tsx"}`.

9. `patch_symbol`:
   - Surgically replaces a named AST symbol with new code.
   - **Usage:** `{"source": "...", "symbol": "login", "new_code": "..."}`.

10. `index_graph`:
    - Indexes workspace into a cross-file SymbolGraph and reports architectural health (cycles & orphan exports).
    - **Usage:** `{"path": "src/"}`.
