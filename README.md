# locus-engine

A high-performance, deterministic AST verification, cross-file semantic symbol graph, and surgical patching engine written in pure Rust (Zero unsafe blocks).

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)
[![Tests: 20/20 Passing](https://img.shields.io/badge/Tests-20%2F20%20Passing-brightgreen.svg)](Cargo.toml)

---

## ⚡ Key Features

- **AstGuard (6-Pass Invariant Verification):** Verifies delimiter balance, division-by-zero bounds, array indexing safety, unsafe unwraps, sync mutexes held across await points, ReDoS backtracking, and deep null dereferencing in `<0.05ms`.
- **SymbolGraph:** Cross-file polyglot AST parser and dependency graph for Rust, TypeScript, JavaScript, and Python with lazy static regex caching.
- **AstContextCache:** In-memory LRU cache keyed by pure Rust FIPS 180-4 standard SHA-256 digests (Zero external crypto dependencies).
- **AstDiffEngine:** Surgical byte-span AST patching and type skeletonization.
- **Pure Headless Systems Crate:** Zero UI/webview bloat, zero background telemetry, and minimal dependency footprint.

---

## 🚀 CLI Usage

### 1. Invariant Safety Check
```bash
locus check <file_path>
```

### 2. Polyglot Symbol Graph Indexing
```bash
locus graph <directory_path>
```

### 3. Surgical Symbol Patching
```bash
locus patch <file_path> --symbol <symbol_name> --with <new_code>
```

---

## 🛠️ Library Integration

Add to your `Cargo.toml`:
```toml
[dependencies]
locus-engine = { path = "../locus-engine" }
```

```rust
use locus_engine::{AstGuard, AstDiffEngine, SymbolGraph, AstContextCache, Language};

fn main() {
    let code = "pub fn safe_calc(a: f64, b: f64) -> f64 { if b != 0.0 { a / b } else { 0.0 } }";
    
    // 1. Verify Invariants
    let report = AstGuard::verify(code);
    assert!(report.passed);

    // 2. Extract Skeleton
    let skeleton = AstDiffEngine::skeletonize(code, Language::Rust);
    println!("Skeleton: {}", skeleton);
}
```

---

## 🧪 Testing & Verification

```bash
cargo test -- --nocapture
```
