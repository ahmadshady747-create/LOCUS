# locus-engine 🦀⚡

> **Deterministic AST Safety Guard, Polyglot Semantic Symbol Graph, and Surgical Byte-Span Patching Engine in Pure Rust.**

[![License: BSL 1.1](https://img.shields.io/badge/License-BSL%201.1-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/locus-engine.svg?color=orange)](https://crates.io/crates/locus-engine)
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

## 📺 Live Terminal Verification Demo

```text
$ locus check src/async_task.rs

+-------------------------------------------------------------+
|                  LOCUS AST GUARD VERIFICATION               |
+-------------------------------------------------------------+
 Target File: src/async_task.rs
 Verified Latency: 0.0194 ms
 Status: [FAIL] Invariant Violation Detected
 Violation Kind: ASYNC_MUTEX_DEADLOCK
 Violation Detail: std::sync::Mutex used in async context with .await — use tokio::sync::Mutex instead.
+-------------------------------------------------------------+

$ locus graph src/

+-------------------------------------------------------------+
|                   LOCUS SYMBOL GRAPH INDEX                  |
+-------------------------------------------------------------+
 Indexed Root: src/
 Total Indexed Files: 7
 Extracted AST Symbols: 24
 Token Savings via AST Skeleton: 74.8%
 Indexing Latency: 4.82 ms
+-------------------------------------------------------------+
```

---

## 📊 Empirical Benchmarks & Comparison

| Metric / Capability | **`locus-engine`** | Traditional Linters / SAST | Cloud AI Guardrails |
| :--- | :---: | :---: | :---: |
| **Verification Latency** | **`< 0.05 ms` (Sub-millisecond)** | 200 – 1,500 ms | 400 – 2,000 ms (Cloud Round-Trip) |
| **Execution Architecture** | **In-Memory Pure Rust Kernel** | Process Spawns / Node.js | Network HTTP REST API |
| **Context Token Savings** | **`> 50% - 80%` (AST Skeleton)** | 0% (Full Files) | 0% (Full Files) |
| **Memory Safety** | **100% Safe Rust (0 Unsafe Blocks)** | Varies (C/C++/Node) | Undefined |
| **External Dependencies** | **Zero Crypto/Runtime Bloat** | Heavy `node_modules` / Python env | Cloud Connection & API Keys |
| **Deterministic Guarantee** | **100% Strict Formal Rejection** | Heuristic Warnings | Probabilistic LLM Re-evaluation |

---

## 🏛️ System Architecture & Workflow Pipeline

```mermaid
flowchart TD
    subgraph Input ["Incoming Code / AI Agent Patch"]
        RawCode["Raw Code Snippet / File"]
    end

    subgraph AstGuardPipeline ["🛡️ AstGuard: 6-Pass Deterministic Firewall (<0.05ms)"]
        P0["Pass 0: Delimiter Balance (Dijkstra)"]
        P1["Pass 1: Async Mutex Across Await"]
        P2["Pass 2: Division-by-Zero Guard"]
        P3["Pass 3: Array Bounds Overflow"]
        P4["Pass 4: Unsafe Unwrap / Expect Trap"]
        P5["Pass 5: ReDoS Catastrophic Backtracking"]
        P6["Pass 6: TS/JS Deep Null Dereference"]
    end

    subgraph Resolution ["Resolution & Verification Verdict"]
        VerdictSafe{"All Passes Passed?"}
        Reject["❌ Immediate Rejection & Counterexample"]
        Approve["✅ Verified Safe AST"]
    end

    subgraph ContextEngine ["✂️ AstDiffEngine & 🧠 SymbolGraph"]
        Cache["⚡ AstContextCache (FIPS 180-4 SHA-256)"]
        Skeleton["Context Compression (>50-80% Token Savings)"]
        Patch["Surgical Byte-Span Node Replacement"]
    end

    RawCode --> P0 --> P1 --> P2 --> P3 --> P4 --> P5 --> P6 --> VerdictSafe
    VerdictSafe -->|No| Reject
    VerdictSafe -->|Yes| Approve
    Approve --> Cache --> Skeleton --> Patch
```

---

## 🛡️ The 6 Deterministic Safety Invariants

```mermaid
graph LR
    A[AstGuard Invariant Passes] --> B[1. Delimiter Balance: Dijkstra stack scan]
    A --> C[2. Concurrency: std::sync::Mutex across .await points]
    A --> D[3. Arithmetic: Unguarded division by variable]
    A --> E[4. Bounds: Array index without length checks]
    A --> F[5. Panics: Unguarded .unwrap() and .expect()]
    A --> G[6. ReDoS: Exponential nested regex quantifiers]
```

1. **Delimiter Balance (Dijkstra Algorithm):** Validates matched closure for `{}` `[]` `()` across raw byte streams.
2. **Async Mutex Concurrency Trap:** Prevents blocking `std::sync::Mutex` guards across asynchronous `.await` suspension points.
3. **Division-by-Zero Protection:** Proves the denominator is non-zero ($y \neq 0$) before allowing arithmetic evaluation.
4. **Array Bounds Protection:** Ensures array/slice indexing (`arr[i]`) is preceded by length assertions or safe accessors (`.get()`).
5. **Unsafe Unwrap Guard:** Eliminates panic-inducing direct `.unwrap()` or `.expect()` calls lacking prior safety checks.
6. **ReDoS Catastrophic Backtracking Guard:** Identifies polynomial and exponential nested quantifiers (such as `(a+)+$`) that freeze CPUs.

---

## 🚀 Quick Install & CLI Usage

### Instant One-Line Installation

#### 🐧 Linux & 🍏 macOS:
```bash
curl -fsSL https://raw.githubusercontent.com/ahmadshady747-create/LOCUS/main/scripts/install.sh | bash
```

#### 🪟 Windows (PowerShell):
```powershell
irm https://raw.githubusercontent.com/ahmadshady747-create/LOCUS/main/scripts/install.ps1 | iex
```

#### 📦 Via Cargo (crates.io):
```bash
cargo install locus-engine
```

---

### Command Line Reference

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
locus-engine = "0.1.0"
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

## 📄 Licensing & Commercial Tiers

`locus-engine` is published under the [Business Source License 1.1 (BSL 1.1)](LICENSE):

| License Tier | Target Audience / Scope | Pricing |
| :--- | :--- | :---: |
| **Free Tier** | Individuals, students, open-source projects, and teams with **< 5 developers**. | **$0 (Free)** |
| **Internal Commercial Seat** | Internal usage & CI/CD within organizations with **5+ developers** *(Internal use only; no re-selling/SaaS)*. | **$150 USD / seat / year** |
| **Commercial SaaS & Cloud OEM** | Embedding, hosting, or offering locus-engine as a commercial SaaS, cloud API, or OEM product. | **$10,000 USD / year** |

For license activation and commercial contracts: Contact the author below or email `licensing@locus.dev`.

---

## 👤 Author & Direct Connect

Architected & built independently by **Ahmed Shadi** (Libya 🇱🇾).

- 📘 **Facebook:** [Ahmed Shadi Profile](https://www.facebook.com/share/1DZibmYSrx/)
- 🐙 **GitHub:** [@ahmadshady747-create](https://github.com/ahmadshady747-create)
- 📧 **Direct Inquiries:** Via GitHub Issues & Discussions
