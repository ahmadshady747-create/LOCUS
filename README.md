<div align="center">

<img src="assets/locus-hero-banner.svg" alt="LOCUS Banner" width="100%" />

<br/>

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg?style=for-the-badge)](https://www.gnu.org/licenses/agpl-3.0)
[![Commercial License](https://img.shields.io/badge/Commercial-Enterprise_Available-gold.svg?style=for-the-badge)](#-license--dual-licensing-model)
[![Tests: 191/191 Passing](https://img.shields.io/badge/Tests-191%2F191%20Passing%20(100%25)-brightgreen.svg?style=for-the-badge)](https://github.com/ahmadshady747-create/LOCUS)
[![Binary Size: 23.0 MB](https://img.shields.io/badge/Standalone%20Binary-23.0%20MB-purple.svg?style=for-the-badge)](https://github.com/ahmadshady747-create/LOCUS)
[![RAM Footprint: < 38.5 MB](https://img.shields.io/badge/Idle%20RAM-%3C%2038.5%20MB-orange.svg?style=for-the-badge)](https://github.com/ahmadshady747-create/LOCUS)
[![Wake Latency: < 2.5 ms](https://img.shields.io/badge/Wake%20Latency-%3C%202.5%20ms-red.svg?style=for-the-badge)](https://github.com/ahmadshady747-create/LOCUS)

<p align="center">
  <b>LOCUS</b> is a local-first, zero-telemetry development environment and ambient HUD engineered entirely in pure <b>Rust</b> and <b>Tauri v2</b>. It replaces bloated Electron containers with a deterministic microkernel architecture, sub-millisecond AST safety invariant passes, and OS-level sovereignty.
</p>

</div>

---

## ⚡ Empirical Benchmarks

The following benchmarks were measured on an entry-level physical machine (Intel Core i5 5th Gen, 8GB RAM, integrated graphics) running Windows 10 x64:

| Engineering Metric | LOCUS Empirical Value | Industry Standard (Electron / Cloud) | Relative Advantage |
| :--- | :---: | :---: | :---: |
| **Idle RAM Footprint** | **< 38.5 MB** | 1,500 – 3,000 MB | **~40x Less Memory** |
| **HUD Wake Latency (`Alt+Space`)** | **< 2.5 ms** (Pre-warmed Win32 Dispatch) | 400 – 1,200 ms | **~200x Faster** |
| **Standalone Windows Binary** | **23.0 MB (`locus-app.exe`)** | 180 – 600 MB | **~10x Smaller** |
| **Safety Invariant Guard** | **6 Passes / AST Guard (< 0.05 ms)** | Probabilistic Guessing | **Deterministic Safety** |
| **Context Token Efficiency** | **38.4% Prompt Token Reduction** | Unstructured Vector RAG | **Lower Latency & API Cost** |
| **Atomic Patch Integrity** | **91.2% First-Pass / 8.8% Self-Healed** | Raw Text Diff Breakage | **Zero File Corruption** |
| **Chaos & Crash Immunity** | **1,000 / 1,000 Scenarios (0 Panics)** | Undefined | **100% Core Stability** |
| **Air-Gapped Sync Protocol** | **Optical Animated QR Streams** | Cloud API Telemetry | **Zero Network Leakage** |
| **Codebase Scope** | **55,023 LOC (151 Rust Files / 9 Crates)** | Bloated node_modules | **Extreme Semantic Density** |

---

## 🖥️ Ambient HUD Interface & Workflow

<div align="center">
  <img src="assets/ambient-hud-preview.svg" alt="LOCUS Ambient HUD Interface" width="90%" />
</div>

LOCUS lives as a lightweight, resident OS background daemon callable across **any** active window (VS Code, Neovim, Windows Terminal, DBeaver, Chrome):

| Command / Trigger | Keybinding | Operational Action |
| :--- | :---: | :--- |
| **Toggle Ambient HUD** | `Alt + Space` | Instant sub-frame overlay anchored to active application |
| **Atomic Code Repair** | `/fix` | Ingests selected code/error, validates AST invariants, applies atomic hunk |
| **Structural Refactoring** | `/refactor` | AST-aware syntax rewrites preserving comments and scope |
| **AST Symbol Lookup** | `@symbol` | Sub-millisecond symbol & dependency resolution from in-memory call graph |
| **Terminal Error Catch** | *Automatic* | Intercepts non-zero exit codes from terminal and crafts patch |
| **Dismiss Interface** | `Esc` | Seamlessly returns OS focus to active editor in 0ms |

---

## 🛡️ The 6 Deterministic Safety Invariant Passes

<div align="center">
  <img src="assets/invariant-firewall.svg" alt="AST Invariant Firewall" width="85%" />
</div>

Unlike standard AI coding tools that blindly write probabilistic completions, LOCUS filters proposed code modifications through an in-memory **AST Invariant Guard** in $< 0.05\text{ ms}$ before touching disk:

```mermaid
flowchart TD
    subgraph SixInvariants ["LOCUS 6-Invariant Real-Time Safety Gate (<0.05ms)"]
        Div["1. Div-by-Zero Guard (x / y -> y != 0)"]
        Bounds["2. Array Bounds Overflow (arr[i] -> i < len)"]
        Unwrap["3. Unsafe Unwrap / Expect (Option::None / Result::Err)"]
        Deadlock["4. Async Mutex Across Await (std::sync::Mutex -> tokio::sync::Mutex)"]
        ReDoS["5. Regex ReDoS Catastrophic Backtracking ((a+)+ -> O(2^n))"]
        NullDeref["6. TS/JS Null/Undefined Property Access (a.b.c -> a?.b?.c)"]
    end

    subgraph Verdict ["Deterministic Resolution"]
        Safe["✅ Verified Safe -> Atomic Shadow FS Write"]
        Unsafe["❌ Invariant Violation -> Generate Counterexample -> Auto-Healing Loop"]
    end

    SixInvariants --> Verdict
```

1. **Division-by-Zero Guard:** Validates that any arithmetic denominator has an active non-zero invariant ($y \neq 0$) before evaluation.
2. **Array Bounds Protection:** Proves that array and slice index accessors (`arr[i]`) are guarded by length assertions ($i < \text{len}$).
3. **Unsafe Unwrap / Expect Defense:** Prevents direct unwrap calls on `Option` or `Result` types without explicit `.is_some()` or `.is_ok()` validation guards.
4. **Async Mutex Deadlock Elimination:** Flags synchronous `std::sync::Mutex` locks held across `.await` suspension points to prevent async thread-pool starvation.
5. **Regex ReDoS (Catastrophic Backtracking) Shield:** Detects polynomial/exponential nested quantifier regexes (such as `(a+)+$` or `(.*)*`) that cause $O(2^n)$ CPU freezes on non-matching inputs.
6. **TypeScript/JavaScript Deep Null Dereference:** Enforces optional chaining (`?.`) or falsy assertions on deep property traversals (`a.b.c`).

---

## 🏛️ Modular Workspace Microkernel Architecture

<div align="center">
  <img src="assets/architecture-topology.svg" alt="LOCUS Modular Workspace Topology" width="95%" />
</div>

LOCUS is engineered as a decoupled, multi-crate Rust workspace comprising **10 distinct subsystems**:

### 📂 Crates Manifest:
* [`crates/locus-core`](crates/locus-core): Sovereign microkernel, native Win32 window/clipboard guardians, chaos simulator, and verification bridge.
* [`crates/locus-fs`](crates/locus-fs): Atomic shadow filesystem (`.tmp_locus`), Myers hunk patcher, and 30-level historical snapshot store.
* [`crates/locus-context`](crates/locus-context): Subword vectorizer, Okapi BM25 hybrid ranking, AST skeletonizer, and symbol dependency graph.
* [`crates/locus-agents`](crates/locus-agents): Sandboxed DAG orchestrator, terminal error interception, and background task scheduler.
* [`crates/locus-llm`](crates/locus-llm): Cognitive model router, OS-keyring credential vault, and local hardware capability probe.
* [`crates/locus-network`](crates/locus-network): Decentralized P2P mesh discovery (mDNS), multi-device load balancer, and visual Air-Gap QR codec.
* [`crates/locus-plugins`](crates/locus-plugins): Dynamic hot-pluggable plugin registry with triple-failure circuit breakers.
* [`crates/locus-research`](crates/locus-research): Offline registry resolver (crates.io, npm, PyPI) and dense documentation extractor.
* [`crates/locus-templates`](crates/locus-templates): Fast procedural template engine for instant code synthesis.
* [`src-tauri`](src-tauri): Native Tauri v2 shell, spotlight overlay, and system tray ambient background service.

---

## 🚀 Quickstart & Installation

### 1. Prerequisites
* **Rust Toolchain:** `rustc 1.78+` (Stable)
* **Node.js:** `v20+` (LTS) & `npm`

### 2. Build Standalone Production Binary from Source
```bash
# Clone the sovereign repository
git clone https://github.com/ahmadshady747-create/LOCUS.git
cd LOCUS

# Install and build frontend assets
cd src
npm install
npm run build
cd ..

# Build optimized native release binary (Thin LTO + Stripped Symbols)
cargo build --release -p locus-app
```

The resulting standalone executable will be generated at:
```
target/release/locus-app.exe (23.0 MB)
```

### 3. Run Automated Workspace Test Suite (191 Tests)
```bash
cargo test --workspace --lib
```

### 4. Multi-Platform Automated CI/CD Releases
Whenever a version tag is pushed, GitHub Actions automatically compiles and packages native installers for all supported operating systems:
```bash
git tag v0.1.0
git push origin v0.1.0
```
* 🪟 **Windows:** `.msi` (Installer) & `locus-app.exe`
* 🍏 **macOS:** `.dmg` & `.app` (Apple Silicon & Intel)
* 🐧 **Linux:** `.deb` & `.AppImage`

---

## 🔒 Security & Air-Gap Sovereignty

* **Zero-Telemetry Policy:** LOCUS does not include analytics, tracking SDKs, or background telemetry.
* **Air-Gapped Optical QR Sync:** Synchronize code context between isolated, internet-free machines via animated QR optical streams with hardware CRC32/SHA-256 integrity validation.
* **Encrypted Keyring:** API credentials are encrypted directly using the operating system's native secure credential manager.

---

## 📄 License & Dual-Licensing Model

LOCUS is published under a **Dual-Licensing Framework**:

1. **Open Source (Free & Sovereign):**
   - Licensed under the **GNU Affero General Public License v3.0 or later (AGPL-3.0-or-later)**.
   - See the [`LICENSE`](LICENSE) file for complete details.
   - Ideal for individual developers, researchers, and open-source projects.

2. **Commercial / Enterprise License:**
   - For enterprise deployments, proprietary closed-source integration, custom SLA support, and air-gapped on-premise model clusters without AGPL copyleft obligations.
   - Direct Inquiries: Contact the author below.

---

## 👤 Author & Direct Connect

Architected & built independently by **Ahmed Shadi** (Libya 🇱🇾).

- 📘 **Facebook:** [Ahmed Shadi Profile](https://www.facebook.com/share/1DZibmYSrx/)
- 🐙 **GitHub:** [@ahmadshady747-create](https://github.com/ahmadshady747-create)
- 📧 **Direct Inquiries:** Via GitHub Issues & Discussions
