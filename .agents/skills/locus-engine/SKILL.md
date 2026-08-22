---
name: locus-engine
description: >-
  Deterministic AST safety verification, surgical symbol patching, context compression,
  cross-file dependency graph resolver, blast-radius impact analyzer, and cross-module symbol resolution powered by the native locus-engine MCP server (v0.3.1).
  Activate this skill when generating, reviewing, patching, or analyzing code in Rust, TypeScript, TSX, JSX, Svelte, Astro, Vue, or Python.
---

# LOCUS Engine Skill Guide (v0.3.1)

Use this skill to leverage the native `locus` MCP server and CLI for sub-millisecond invariant safety verification, blast-radius impact analysis, and token-optimized code analysis across Backend (Rust, Python) and Frontend (TSX, JSX, Svelte, Astro, Vue) ecosystems.

## Available MCP Tools:

1. `get_blast_radius`:
   - **When to use:** **CRITICAL BEFORE REFACTORING**. Call this tool before modifying or deleting any existing function, struct, interface, or component to calculate downstream breaking change risk, identify all direct and transitive dependent files, and get the exact impact radius.
   - **Usage:** Call `get_blast_radius` with `{"symbol": "AstGuard", "path": "src/", "depth": 2}`.

2. `resolve_symbol`:
   - **When to use:** When you need the exact origin file, byte coordinates, type signature, and doc-comments for a symbol across multi-file module imports.
   - **Usage:** Call `resolve_symbol` with `{"symbol": "UserProfileCard", "from_file": "src/Dashboard.tsx", "target_path": "src/"}`.

3. `find_references`:
   - **When to use:** To locate all call sites, imports, and usages of a symbol across the entire workspace before renaming or changing parameters.
   - **Usage:** Call `find_references` with `{"symbol": "check_safety", "target_path": "src/"}`.

4. `synthesize_contract`:
   - **When to use:** **PROACTIVE STEP (Before generating code)**. Pass the developer's intent and target path to generate strict type scaffolding (request/response structs, error enums, trait bounds, interfaces) and invariant checklists.
   - **Usage:** Call `synthesize_contract` with `{"intent": "...", "target_path": "src/auth.rs", "language": "rust|tsx|python"}`.

5. `extract_intent_slice`:
   - **When to use:** When you need a minimal, high-density AST context slice containing *only* the target symbol and its direct dependencies up to N degrees of separation, eliminating 100% of architectural noise.
   - **Usage:** Call `extract_intent_slice` with `{"symbol": "UserProfileCard", "code": "...", "depth": 2}` or `{"symbol": "...", "path": "src/"}`.

6. `verify_contract`:
   - **When to use:** **BIDIRECTIONAL VERIFICATION STEP (After generating code)**. Verifies that generated code strictly preserves agreed signatures, types, and implements all requirements with zero safety violations.
   - **Usage:** Call `verify_contract` with `{"intent": "...", "generated_code": "...", "language": "rust|tsx|python"}`.

7. `check_safety`:
   - **When to use:** Before proposing or committing any code changes in Rust, TSX, JSX, Svelte, Astro, Vue, TypeScript, JavaScript, or Python.
   - **Invariants Checked:**
     - Dijkstra delimiter balance (`{}`, `[]`, `()`)
     - Dijkstra JSX/HTML tag balancing (opening/closing matching, fragments `<>...</>`, self-closing void elements)
     - React Rules of Hooks (detects conditional `useState`/`useEffect`/`use*` inside `if`, loops, or ternary expressions)
     - Client/Server Secret Leak Guard (detects un-prefixed secrets like `process.env.DATABASE_URL` in `"use client"` files)
     - Unsafe raw HTML injection (`dangerouslySetInnerHTML` without sanitization)
     - Async mutex deadlocks (`std::sync::Mutex` held across `.await`)
     - Unguarded division by variable (`x / y` without `y != 0`)
     - Array out-of-bounds indexing
     - Direct unsafe unwraps (`.unwrap()`, `.expect()`)
     - ReDoS catastrophic backtracking patterns
     - Deep property access without optional chaining (`?.`)
   - **Usage:** Call `check_safety` with `{"code": "..."}` or `{"path": "src/Component.tsx"}`.

8. `skeletonize`:
   - **When to use:** When you need to inspect component architecture, types, or API signatures without wasting token budget on JSX render trees or large implementation bodies (>70-85% token reduction).
   - **Usage:** Call `skeletonize` with `{"code": "...", "language": "rust|typescript|javascript|tsx|jsx|svelte|astro|vue|python"}`.

9. `patch_symbol`:
   - **When to use:** When making surgical updates to a single function, struct, component, or event handler (e.g. `handleSubmit`) without regenerating whole files.
   - **Usage:** Call `patch_symbol` with `{"source": "...", "symbol": "handleSubmit", "new_code": "...", "language": "tsx"}`.

10. `index_graph`:
    - **When to use:** When mapping symbol definitions, component exports, hooks, imports, traits, and call hierarchies across an entire project directory.
    - **Usage:** Call `index_graph` with `{"path": "src/"}`.
