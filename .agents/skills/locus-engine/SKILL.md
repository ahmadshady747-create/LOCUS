---
name: locus-engine
description: >-
  Deterministic AST safety verification, surgical symbol patching, context compression,
  and cross-file dependency graph resolver powered by the native locus-engine MCP server (v0.3.0).
  Activate this skill when generating, reviewing, patching, or analyzing code in Rust, TypeScript, TSX, JSX, Svelte, Astro, Vue, or Python.
---

# LOCUS Engine Skill Guide (v0.3.0)

Use this skill to leverage the native `locus` MCP server and CLI for sub-millisecond invariant safety verification and token-optimized code analysis across Backend (Rust, Python) and Frontend (TSX, JSX, Svelte, Astro, Vue) ecosystems.

## Available MCP Tools:

1. `check_safety`:
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

2. `skeletonize`:
   - **When to use:** When you need to inspect component architecture, types, or API signatures without wasting token budget on JSX render trees or large implementation bodies (>70-85% token reduction).
   - **Usage:** Call `skeletonize` with `{"code": "...", "language": "rust|typescript|javascript|tsx|jsx|svelte|astro|vue|python"}`.

3. `patch_symbol`:
   - **When to use:** When making surgical updates to a single function, struct, component, or event handler (e.g. `handleSubmit`) without regenerating whole files.
   - **Usage:** Call `patch_symbol` with `{"source": "...", "symbol": "handleSubmit", "new_code": "...", "language": "tsx"}`.

4. `index_graph`:
   - **When to use:** When mapping symbol definitions, component exports, hooks, imports, traits, and call hierarchies across an entire project directory.
   - **Usage:** Call `index_graph` with `{"path": "src/"}`.
