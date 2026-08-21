# locus-engine Formal Technical Specification

**Version:** 0.2.0  
**License:** Business Source License 1.1 (BSL 1.1)  
**Author:** Ahmed Shadi (Libya 🇱🇾)  
**Architecture:** Headless Pure-Rust Systems Crate, CLI & Stdio MCP Server  

---

## 1. Abstract & Scope

`locus-engine` is a high-speed systems engine written in 100% safe Rust (`zero unsafe blocks`). It provides:
1. **Deterministic Invariant AST Verification (`AstGuard`):** Microsecond-latency safety firewalls enforcing 6 structural and concurrency rules on code transformations.
2. **Context Window Compression (`AstDiffEngine`):** Surgical type-skeleton extraction stripping function bodies while preserving signatures, reducing LLM context token inflation by >50–80%.
3. **Polyglot Semantic Symbol Graph (`SymbolGraph`):** Content-addressed index of functions, structs, traits, and modules across Rust, TypeScript, and Python.
4. **FIPS 180-4 In-Memory LRU Cache (`AstContextCache`):** Monotonically indexed in-memory cache keyed by custom pure-Rust SHA-256 digests.
5. **Model Context Protocol Server (`mcp`):** Native JSON-RPC 2.0 stdio server communicating with modern AI IDEs (Claude Code, Cursor, Windsurf).

---

## 2. Formal Invariant Specifications (`AstGuard`)

### Invariant 0: Delimiter Balance ($\mathcal{I}_0$)
- **Algorithm:** Linear single-pass stack scan ($O(N)$).
- **Rule:** For any source string $S$, matched pairs of `{}` `[]` `()` must resolve to an empty stack upon EOF, skipping string literals (`""`, `''`) and character escapes (`\"`).
- **Violation:** `UNBALANCED_DELIMITERS`.

### Invariant 1: Concurrency Safety ($\mathcal{I}_1$)
- **Rule:** Standard synchronous mutexes (`std::sync::Mutex`) must not be held across asynchronous suspension points (`.await`).
- **Violation:** `ASYNC_MUTEX_DEADLOCK`.

### Invariant 2: Arithmetic Safety ($\mathcal{I}_2$)
- **Rule:** For any variable division $a / b$, the denominator $b$ must be preceded by a non-zero guard ($b \neq 0$) or assertion (`assert!`) within the lexical scope. Numeric literal divisors ($a / 2$) are explicitly permitted.
- **Violation:** `DIV_BY_ZERO`.

### Invariant 3: Memory & Index Bounds ($\mathcal{I}_3$)
- **Rule:** Direct variable index accesses $arr[i]$ must be bounded by length checks (`.len()`, `.is_empty()`) or use safe accessors (`.get()`).
- **Violation:** `ARRAY_OOB`.

### Invariant 4: Panic-Free Extraction ($\mathcal{I}_4$)
- **Rule:** Direct invocations of `.unwrap()` or `.expect()` without prior safety verification (`is_some()`, `is_ok()`, or `if let`) are forbidden.
- **Violation:** `UNSAFE_UNWRAP`.

### Invariant 5: Regular Expression Polynomial Backtracking ($\mathcal{I}_5$)
- **Rule:** Regular expressions with nested quantifiers (e.g. `(a+)+$`) that exhibit exponential backtracking ($O(2^n)$) are formally rejected.
- **Violation:** `REDOS_PATTERN`.

---

## 3. Data Structures & Memory Layout

```rust
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Const,
    Impl,
    Module,
}

pub struct SymbolNode {
    pub id: u64,             // FNV-1a 64-bit content hash
    pub name: String,
    pub kind: SymbolKind,
    pub file: String,
    pub byte_start: usize,
    pub byte_end: usize,
    pub signature: String,
}

pub struct VerificationReport {
    pub passed: bool,
    pub violation: Option<ViolationKind>,
    pub detail: Option<String>,
    pub latency_ms: f64,
}
```

---

## 4. MCP Protocol Specification

The engine exposes a JSON-RPC 2.0 stdio server adhering to the Model Context Protocol (version `2024-11-05`):

### Supported Methods:
- `initialize`: Returns server info (`locus-engine 0.1.0`) and tool capabilities.
- `notifications/initialized`: Acknowledges client readiness.
- `ping`: Liveness check returning `{}`.
- `tools/list`: Exposes `check_safety`, `skeletonize`, `patch_symbol`, `index_graph`.
- `tools/call`: Dispatches parameter payload to native Rust algorithms and returns standard text content blocks.

---

## 5. Benchmarks & Validation Results

- **Compiler Profile:** `Rust 1.80+ (opt-level = 3, lto = "thin")`
- **AstGuard Verification Latency:** `9.04 µs` (average across 1,000 cycles)
- **AstContextCache SHA-256:** `18.68 µs` (average across 1,000 cycles)
- **AstDiffEngine Patching:** `56.13 µs` (average across 500 cycles)
- **SymbolGraph Indexing:** `16.29 ms` (600 polyglot files, 1,600 symbols)
- **MCP Stdio Dispatch:** `42.32 µs` (average across 1,000 cycles)
- **Test Suite Status:** `31/31 Passed (100% assertions valid)`
- **Unsafe Code Audit:** `0 Unsafe Blocks`
