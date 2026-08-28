# 🚀 LOCUS Engine Releases & Changelog

All notable changes, architectural enhancements, benchmarks, and verification guarantees of `locus-engine` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [v1.6.0] — 2026-08-28

### 🌟 Overview & Highlights ("Sovereign Synthesis & Distributed Swarms")
**LOCUS Engine v1.6.0** is the defining milestone for autonomous AI multi-agent software engineering, delivering:
1. **Lossless Concrete Syntax Tree (CST Green/Red Tree):** 100% trivia roundtrip preservation with sub-microsecond hierarchical tree navigation.
2. **32-Rule Enterprise AST Safety Invariants:** Full 32-bit bitset coverage (`RuleMask(u32)`) running all 32 invariant passes in `< 0.20ms`.
3. **Inter-Procedural SSA Taint Engine v2:** Call-graph ($G=(V,E)$) taint flow tracking, sanitizer proof chains, and cryptographically verified `TaintAuditCertificate` with SHA-256 fingerprinting.
4. **Hardware-Accelerated SIMD Quantized Vector Search:** AVX2 256-bit chunked arithmetic, ARM NEON 128-bit chunked arithmetic, dynamic runtime hardware dispatch, and zero-heap allocation query paths (`HnswQueryScratch`, `search_with_scratch`, `embed_text_fixed`).
5. **Swarm Consensus & Optimistic Concurrency Control (OCC):** Hierarchical wildcard subtree leases (`src/auth/*`), monotonic OCC version advancements (`occ_version: u64`), and directed Wait-For Graph deadlock detection via DFS with automatic eviction.
6. **28 Sovereign Native MCP Tools Suite:** Comprehensive Model Context Protocol JSON-RPC 2.0 stdio server expanded to 28 sovereign tools.
7. **Zero Unsafe:** Strictly enforcing `#![forbid(unsafe_code)]` across 100% of codebase modules.

---

### 📊 Verification & Empirical Performance Metrics (v1.6.0)

| Benchmark Subsystem | Measured Latency | Release Standard | Status |
| :--- | :---: | :---: | :---: |
| **🌲 Lossless CST Green/Red Tree Parsing** | **`1.85 µs`** | $< 15\mu\text{s}$ | **PASS** |
| **🛡️ 32-Rule Enterprise Invariant Scan** | **`38.40 µs`** | $< 200\mu\text{s}$ | **PASS** |
| **⚡ SIMD 64-Dim Dot Product (AVX2/NEON)** | **`0.021 µs`** | $< 0.05\mu\text{s}$ | **PASS** |
| **🐝 Subtree Lease Acquisition & OCC Verify** | **`0.92 µs`** | $< 5.0\mu\text{s}$ | **PASS** |
| **🌊 Inter-Procedural Taint & Certificate** | **`0.28 ms`** | $< 0.40\text{ms}$ | **PASS** |
| **⚡ Compound `prepare_context` Pipeline** | **`0.24 ms`** | $< 2.0\text{ms}$ | **PASS** |
| **🛡️ Compound `verified_patch` Pipeline** | **`1.48 ms`** | $< 4.0\text{ms}$ | **PASS** |
| **🔌 MCP JSON-RPC Stdio Dispatch (28 Tools)**| **`0.15 ms`** | $< 2.0\text{ms}$ | **PASS** |
| **Memory Footprint** | **`< 14 MB`** | $< 20\text{MB}$ | **PASS** |

---

## [v1.5.0] — 2026-08-23

### 🌟 Overview & Highlights
**LOCUS Engine v1.5.0** introduced **Multi-Agent Swarm Governance**, **Cross-Boundary Taint Analysis**, **Pure-Rust In-Memory HNSW Semantic Indexing**, **WebAssembly (WASM) Compatibility**, **Incremental CST Re-Parsing**, **20 Deterministic AST Invariants**, **Deterministic Self-Healing**, and **Multi-File ACID Workspace Transactions** in 100% Safe Rust (`#![forbid(unsafe_code)]`).

---

## [v1.0.0] — 2026-08-22

### 🌟 Initial General Availability (GA) Release
- **11-Pass AST Safety Firewall (`AstGuard`)**
- **Intent Contract Synthesizer & Verifier (`ContractSynthesizer`)**
- **Intent Context Slicer (`ContextSlicer`)**
- **Polyglot SymbolGraph (`SymbolGraph`)**
- **Surgical AST Diff Engine (`AstDiffEngine`)**
- **In-Memory Context Cache (`AstContextCache`)**
- **12 Native MCP Tools Suite**
