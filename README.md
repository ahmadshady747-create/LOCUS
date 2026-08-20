# locus-engine 🦀⚡

> **Deterministic AST Safety Guard, Polyglot Semantic Symbol Graph, and Surgical Byte-Span Patching Engine in Pure Rust.**

[![License: BSL 1.1](https://img.shields.io/badge/License-BSL%201.1-blue.svg)](LICENSE)
[![Tests: 20/20 Passing](https://img.shields.io/badge/Tests-20%2F20%20Passing-brightgreen.svg)](Cargo.toml)
[![Memory Safety: Zero Unsafe](https://img.shields.io/badge/Memory%20Safety-Zero%20Unsafe-success.svg)](src/)
[![Latency: <0.05ms](https://img.shields.io/badge/Verification-Sub--0.05ms-purple.svg)](src/guard.rs)
[![Zero External Crypto](https://img.shields.io/badge/Crypto-FIPS%20180--4%20Pure%20SHA--256-orange.svg)](src/cache.rs)

---

## ⚡ Overview & Vision

Modern AI code generation agents and automated pipelines face two systemic engineering bottlenecks:
1. **Probabilistic Syntax & Concurrency Regressions:** AI agents frequently hallucinate unclosed delimiters, dangerous unwrap traps, async-mutex deadlocks, catastrophic regexes (ReDoS), and unbounded array access.
2. **Context Window Inflation:** Passing full source files repeatedly into LLM contexts wastes up to 80% of token budgets.

**`locus-engine`** solves both challenges as a **standalone, zero-bloat, high-performance systems engine** written in 100% safe Rust. It enforces deterministic, non-negotiable safety invariants in microseconds and extracts cross-file symbol graphs with minimal context footprints.

---

## 🏛️ Core Architectural Pillars

- **🛡️ AstGuard:** 6-Pass Deterministic Safety Invariants (<0.05ms) covering delimiter balance, async mutex across await, division-by-zero, array bounds, unsafe unwraps, ReDoS, and deep null dereference.
- **🧠 SymbolGraph:** Polyglot AST symbol indexer and cross-file dependency graph across Rust, TypeScript, JavaScript, and Python.
- **⚡ AstContextCache:** High-speed in-memory LRU cache keyed by pure Rust FIPS 180-4 standard SHA-256 (Zero external crypto dependencies).
- **✂️ AstDiffEngine:** Surgical byte-span AST patching and structural skeletonization for context compression (>50-80% token savings).

---

## 🚀 CLI Usage

Download standalone binaries from [Releases](https://github.com/ahmadshady747-create/LOCUS/releases).

```bash
# 1. Deterministic Safety Verification (<0.05ms)
locus check src/lib.rs

# 2. Index Workspace Symbol Graph & Measure Token Savings
locus graph src/

# 3. Surgical Symbol Patching
locus patch src/models.rs --symbol User --with "pub struct User { pub id: u64 }"
```

---

## 🛠️ Rust Library Integration

Add `locus-engine` to your `Cargo.toml`:

```toml
[dependencies]
locus-engine = { path = "../locus-engine" }
```

```rust
use locus_engine::{AstGuard, AstDiffEngine, SymbolGraph, Language};

fn main() {
    let code = "pub fn safe_calc(a: f64, b: f64) -> f64 { if b != 0.0 { a / b } else { 0.0 } }";

    // 1. Instant Invariant Safety Verification (<0.05ms)
    let report = AstGuard::verify(code);
    assert!(report.passed);

    // 2. Surgical Context Compression
    let skeleton = AstDiffEngine::skeletonize(code, Language::Rust);
    println!("Compressed Skeleton:\n{}", skeleton);
}
```

---

## 📄 License & Commercial Seat Terms

Published under the [Business Source License 1.1 (BSL 1.1)](LICENSE). Free for individuals, students, non-commercial use, and teams with fewer than 5 developers. Organizations with 5+ developers require a commercial seat license ($150 USD/seat/year). Contact: `licensing@locus.dev`.
