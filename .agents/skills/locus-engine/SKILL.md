---
name: locus-engine
description: >-
  Deterministic AST safety verification, surgical symbol patching, context compression,
  and cross-file dependency graph resolver powered by the native locus-engine MCP server.
  Activate this skill when generating, reviewing, patching, or analyzing code in Rust, TypeScript, or Python.
---

# LOCUS Engine Skill Guide

Use this skill to leverage the native `locus` MCP server and CLI for sub-millisecond invariant safety verification and token-optimized code analysis.

## Available MCP Tools:

1. `check_safety`:
   - **When to use:** Before proposing or committing any code changes in Rust, TypeScript, JavaScript, or Python.
   - **Checks performed:** Dijkstra delimiter balance, async mutex across await points, unguarded division by variable, array bounds overflow, direct unsafe unwraps, and ReDoS regex patterns.
   - **Usage:** Call `check_safety` with `{"code": "..."}` or `{"path": "src/file.rs"}`.

2. `skeletonize`:
   - **When to use:** When you need to understand the architecture or APIs of large source files without wasting 80% of token budget on implementation bodies.
   - **Usage:** Call `skeletonize` with `{"code": "...", "language": "rust|typescript|python"}`.

3. `patch_symbol`:
   - **When to use:** When making surgical updates to a single function, struct, or class without regenerating whole files.
   - **Usage:** Call `patch_symbol` with `{"source": "...", "symbol": "my_fn", "new_code": "...", "language": "rust"}`.

4. `index_graph`:
   - **When to use:** When mapping symbol definitions, imports, traits, and call hierarchies across an entire project directory.
   - **Usage:** Call `index_graph` with `{"path": "src/"}`.
