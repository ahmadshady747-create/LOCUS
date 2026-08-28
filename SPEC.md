# locus-engine Formal Technical Specification

**Version:** 1.6.0 (Sovereign Synthesis & Distributed Swarms)  
**License:** Business Source License 1.1 (BSL 1.1)  
**Author:** Ahmed Shadi (Libya)  
**Architecture:** Headless Pure-Rust Systems Crate, CLI & Stdio MCP Server (100% Safe Rust #![forbid(unsafe_code)])  

---

## 1. Abstract & Scope

locus-engine (v1.6.0) is an ultra-high-throughput, zero-unsafe systems engine designed to govern autonomous AI coding agents and distributed multi-agent swarms. It enforces deterministic code safety, eliminates hallucinations, compresses context windows, and coordinates multi-agent concurrency in sub-millisecond time across polyglot workspaces (Rust, TypeScript, JavaScript, TSX/JSX, Svelte, Astro, Vue, Python).

Core Subsystems:
1. **Lossless Concrete Syntax Tree (src/cst/):** Pure-Rust Green/Red Tree architecture preserving 100% of formatting, comments, and trivia while enabling sub-microsecond hierarchical tree navigation and token span resolution.
2. **32 Enterprise AST Invariants (src/guard/):** Bitset-accelerated (RuleMask(u32)) deterministic safety firewall checking 32 formal security, memory, and concurrency rules in <0.20ms.
3. **Inter-Procedural SSA Taint Engine v2 (src/taint/):** Cross-file call-graph data flow analyzer verifying sanitizer proof chains and issuing cryptographically signed TaintAuditCertificate tokens with SHA-256 fingerprints.
4. **Hardware-Accelerated SIMD Vector Search (src/search/):** AVX2 256-bit, ARM NEON 128-bit, and zero-heap query scratch buffer (HnswQueryScratch) executing quantized 8-bit semantic search with sub-20µs latency.
5. **Swarm Consensus & Optimistic Concurrency Control (src/lease/):** Hierarchical wildcard subtree leases (crate::auth::*), monotonic OCC version advancement (occ_version), and directed Wait-For Graph deadlock cycle detection via DFS with automatic eviction.
6. **28 Sovereign MCP Tools (src/mcp.rs):** High-throughput JSON-RPC 2.0 stdio server providing 28 native tools for modern AI environments (Claude Code, Cursor, Windsurf, Antigravity).
7. **Multi-File ACID Workspace Transactions (src/tx/):** In-memory shadow buffers validating 100% invariant passes before committing atomically to disk.

---

## 2. Formal Invariant Specifications (AstGuard 32 Enterprise Rules)

| Rule # | Invariant Identifier | Mathematical / Logical Rule | Violation |
| :---: | :--- | :--- | :--- |
| **0** | UNBALANCED_DELIMITERS | Stack depth of paired delimiters {} [] () must equal 0 at EOF. | Delimiter mismatch |
| **1** | ASYNC_MUTEX_DEADLOCK | Synchronous mutex lock held across an async .await suspension point. | Concurrency deadlock |
| **2** | DIV_BY_ZERO | Variable division a / b without preceding lexical non-zero guard (b != 0). | Division by zero |
| **3** | ARRAY_OOB | Variable indexing arr[i] without bounds check or safe accessor (.get()). | Out-of-bounds access |
| **4** | UNSAFE_UNWRAP | Direct invocation of .unwrap() or .expect() on unverified Option/Result. | Panic regression |
| **5** | REDOS_PATTERN | Regex patterns with nested quantifiers yielding exponential backtracking O(2^n). | ReDoS DoS risk |
| **6** | NULL_DEREF | Deep property chaining (a.b.c) without null / optional chaining guards (?.). | Null pointer dereference |
| **7** | CONDITIONAL_HOOK_CALL| React hooks called inside if, loops, or nested callbacks violating Rules of Hooks. | React state race |
| **8** | CLIENT_SECRET_LEAK | Server secret tokens accessed directly inside use client modules. | Secret exposure |
| **9** | UNSAFE_INNER_HTML | Direct unescaped HTML injection via dangerouslySetInnerHTML or .innerHTML. | Cross-site scripting (XSS)|
| **10**| JSX_TAG_MISMATCH | Unmatched or improperly balanced JSX/HTML opening and closing tags. | Malformed markup |
| **11**| SQL_INJECTION | Unparameterized template string interpolation inside SQL query functions. | SQL injection |
| **12**| FLOATING_PROMISE | Unhandled asynchronous Promise lacking await, .catch(), or void. | Unhandled rejection |
| **13**| REACT_STATE_RACE | Non-functional state update inside asynchronous loops or delayed callbacks. | State inconsistency |
| **14**| LISTENER_LEAK | Event listener attached without cleanup in component unmount handler. | Memory leak |
| **15**| INSECURE_RANDOMNESS | Weak PRNG (Math.random) used in cryptographic or token generation scopes. | Insecure randomness |
| **16**| PATH_TRAVERSAL | Unsanitized user inputs concatenated directly into filesystem path variables. | Path traversal |
| **17**| UNBOUNDED_REGEX | Unbounded regex execution risking high-complexity denial of service. | Regex complexity DoS |
| **18**| DYNAMIC_CODE_EVAL | Dynamic code execution via eval(), 
ew Function(), or unvalidated imports. | Arbitrary code eval |
| **19**| UNTYPED_UNION_ACCESS | Polymorphic union property access without type narrowing discriminant checks. | Type safety evasion |
| **20**| CIRCULAR_MEM_LEAK | Reference cycles in Rc<RefCell<T>> or Arc<Mutex<T>> lacking Weak references. | Circular memory leak |
| **21**| ASYNC_CANCELLATION_SAFETY| Non-atomic mutations at asynchronous cancellation suspension points. | Corrupted state |
| **22**| CONSTANT_TIME_CRYPTO | Variable-time comparison in passwords/signatures lacking ConstantTimeEq. | Timing side-channel |
| **23**| EXHAUSTIVE_ENUM_NARROWING| Missing discriminant branches in enum match blocks. | Incomplete match branch |
| **24**| RESOURCE_DESCRIPTOR_LEAK| Sockets, files, or child processes lacking deterministic RAII close/drop. | Descriptor exhaustion |
| **25**| SSRF_UNSAFE_FETCH | Outbound network requests to internal private IP ranges or metadata URLs. | SSRF vulnerability |
| **26**| UNBOUNDED_CHANNEL_DEADLOCK| Unbuffered channel synchronization leading to single-threaded deadlocks. | Channel stall |
| **27**| PROTOTYPE_POLLUTION | Unsanitized object deep-merge exposing __proto__ or constructor. | Prototype pollution |
| **28**| CORS_WILDCARD_CREDENTIALS| Wildcard CORS origin header (*) combined with credentials: include. | CORS security violation |
| **29**| HARDCODED_KEY_ENTROPY | Static high-entropy string tokens representing hardcoded cryptographic secrets. | Exposed static secret |
| **30**| UNCHECKED_ARITHMETIC_OVERFLOW| Unchecked integer operations risking overflow inside critical loops. | Integer overflow |
| **31**| ATOMIC_STATE_MUTATION | Direct non-functional mutations inside high-concurrency state containers. | Concurrent state race |

---

## 3. Lossless Concrete Syntax Tree (Green/Red CST Architecture)

- **Roundtrip Fidelity:** Text(Parse(S)) == S across Rust, TSX, Python.
- **Trivia Preservation:** Comments, docstrings, formatting, newlines, and indentation are fully preserved.
- **Microsecond Navigation:** Absolute byte coordinates and sibling navigation compute in <5µs.

---

## 4. MCP Tools Matrix (28 Sovereign Tools)

1. check_safety
2. skeletonize
3. patch_symbol
4. index_graph
5. synthesize_contract
6. extract_intent_slice
7. erify_contract
8. esolve_symbol
9. get_blast_radius
10. ind_references
11. prepare_context
12. erified_patch
13. egin_tx
14. stage_tx
15. commit_tx
16. ollback_tx
17. uto_remediate
18. cquire_symbol_lease
19. elease_symbol_lease
20. enew_symbol_lease
21. 	race_taint_flow
22. hybrid_search
23. query_cst [NEW v1.6.0]
24. udit_taint_path [NEW v1.6.0]
25. cquire_subtree_lease [NEW v1.6.0]
26. erify_occ_token [NEW v1.6.0]
27. morph_ast [NEW v1.6.0]
28. simd_vector_search [NEW v1.6.0]
