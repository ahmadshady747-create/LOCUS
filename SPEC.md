# LOCUS - Local-First AI Coding Assistant

## Project Overview

**Project Name:** LOCUS (Local Operations & Coding Unified System)  
**Type:** Desktop Application (Tauri v2 + Rust)  
**Core Philosophy:** Local-first, privacy-preserving, zero-cloud AI coding assistant  
**Target Platform:** Windows, macOS, Linux  

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
                        Tauri v2 Frontend (React/TypeScript)       
└─────────────────────────────────────────────────────────────────┘
                              │  IPC Commands
                              ▼
┌─────────────────────────────────────────────────────────────────┐
                      Rust Backend (Tokio Async Runtime)           
├──────────────────┬──────────────────┬────────────────────────────┤
│  FileSystemEngine│  TemplateEngine  │    ContextManager          │
│  (notify, walk)  │  (tera, serde)   │    (prompt assembly)       │
├──────────────────┼──────────────────┼────────────────────────────┤
│ NetworkOrchestrator                │ EphemeralAgentManager      │
│ (mdns-sd, tokio)                   │ (tokio, nix, sandbox)      │
└─────────────────────────────────────────────────────────────────┘
```

---

## Module Specifications

### 1. FileSystemEngine
**Purpose:** Watch, read, write, and modify project files with real-time updates

**Capabilities:**
- Recursive directory scanning with ignore patterns (.gitignore, .locusignore)
- File content reading (text/binary detection)
- File writing with atomic operations
- File modification (patch/diff application)
- Real-time file watching via `notify` crate
- Workspace indexing for fast search

**API Surface:**
```rust
pub struct FileSystemEngine {
    pub async fn scan_workspace(&self, root: &Path) -> Result<WorkspaceIndex>;
    pub async fn read_file(&self, path: &Path) -> Result<FileContent>;
    pub async fn write_file(&self, path: &Path, content: &str) -> Result<()>;
    pub async fn modify_file(&self, path: &Path, ops: &[ModificationOp]) -> Result<()>;
    pub fn watch(&self, paths: &[PathBuf]) -> Result<FileWatchStream>;
    pub async fn search(&self, query: &str) -> Result<Vec<SearchResult>>;
}
```

**Data Structures:**
- `WorkspaceIndex`: HashMap<PathBuf, FileMetadata>
- `FileMetadata`: size, modified, hash, language, symbols
- `ModificationOp`: Insert, Delete, Replace with line/column ranges
- `FileWatchStream`: Stream of `FileEvent` (Created, Modified, Deleted, Renamed)

---

### 2. TemplateEngine
**Purpose:** Manage reference code templates for common patterns (JWT, OAuth, Stripe, etc.)

**Capabilities:**
- Template discovery from `~/.locus/templates/` and project `.locus/templates/`
- Template versioning and metadata (name, description, tags, variables)
- Variable substitution with validation
- Template composition (extends, includes)
- Built-in template library: JWT auth, OAuth2, Stripe payments, GraphQL, REST API, Database migrations, Docker, CI/CD

**API Surface:**
```rust
pub struct TemplateEngine {
    pub async fn list_templates(&self, category: Option<&str>) -> Result<Vec<TemplateInfo>>;
    pub async fn get_template(&self, id: &str) -> Result<Template>;
    pub async fn render(&self, id: &str, vars: &HashMap<String, Value>) -> Result<String>;
    pub async fn install_template(&self, source: TemplateSource) -> Result<TemplateInfo>;
    pub async fn create_template(&self, spec: TemplateSpec) -> Result<TemplateInfo>;
}
```

**Data Structures:**
- `Template`: id, name, description, category, version, variables, files[], dependencies[]
- `TemplateVariable`: name, type, description, default, validation, required
- `TemplateSource`: LocalPath, GitUrl, Registry

**Built-in Templates:**
- `jwt-auth`: RS256/HS256 token generation, validation, refresh
- `oauth2-provider`: Authorization code flow, PKCE, token storage
- `stripe-payments`: Checkout, subscriptions, webhooks, refunds
- `graphql-api`: Schema, resolvers, dataloaders, subscriptions
- `rest-api`: CRUD, pagination, filtering, OpenAPI spec
- `db-migrations`: SQL/SeaORM migrations, seeding, rollback
- `docker-compose`: Multi-service, healthchecks, networks
- `ci-cd`: GitHub Actions, GitLab CI, testing, deployment

---

### 3. ContextManager
**Purpose:** Assemble optimized prompts by combining user request, relevant templates, and project context

**Capabilities:**
- Semantic code search (symbol-based, not just text)
- Relevant file selection based on request intent
- Token budget management (fit within context window)
- Prompt template system with sections
- Context compression (summarization, deduplication)
- Multi-turn conversation context tracking

**API Surface:**
```rust
pub struct ContextManager {
    pub async fn assemble_context(&self, request: &ContextRequest) -> Result<AssembledContext>;
    pub async fn get_relevant_files(&self, query: &str, budget: usize) -> Result<Vec<ContextFile>>;
    pub async fn compress_context(&self, ctx: &AssembledContext, target_tokens: usize) -> Result<AssembledContext>;
    pub fn estimate_tokens(&self, text: &str) -> usize;
}
```

**Data Structures:**
- `ContextRequest`: user_prompt, intent, file_refs, template_refs, max_tokens
- `AssembledContext`: system_prompt, user_prompt, template_context, file_context, metadata
- `ContextFile`: path, content, relevance_score, symbols, token_count

**Prompt Sections:**
1. System prompt (role, capabilities, constraints)
2. Project context (structure, key files, patterns)
3. Template context (selected templates, rendered examples)
4. File context (relevant source files)
5. User request (current task)
6. Instructions (output format, constraints)

---

### 4. NetworkOrchestrator
**Purpose:** Discover local devices via mDNS and distribute LLM inference tasks

**Capabilities:**
- mDNS service discovery (`_locus-llm._tcp.local.`)
- Device capability advertisement (model, VRAM, quantization, context window)
- Task distribution with load balancing
- Peer-to-peer communication (WebRTC data channels or TCP)
- Health monitoring and failover
- Secure local communication (noise protocol or TLS)

**API Surface:**
```rust
pub struct NetworkOrchestrator {
    pub async fn start_discovery(&self) -> Result<()>;
    pub async fn discover_peers(&self) -> Result<Vec<PeerInfo>>;
    pub async fn distribute_task(&self, task: &LLMTask) -> Result<TaskResult>;
    pub async fn register_local_node(&self, caps: NodeCapabilities) -> Result<()>;
    pub fn peer_events(&self) -> PeerEventStream;
}
```

**Data Structures:**
- `PeerInfo`: id, name, address, capabilities, last_seen, status
- `NodeCapabilities`: models[], max_context, vram_gb, quantization[], performance_score
- `LLMTask`: id, prompt, model_preference, temperature, max_tokens, stream
- `TaskResult`: id, response, tokens_used, latency_ms, peer_id

**Protocol:**
- Service: `_locus-llm._tcp.local.`
- TXT records: model=list, vram=24, ctx=32k, quant=q4_k_m
- RPC: JSON-RPC 2.0 over TCP/WebRTC
- Heartbeat: 30s interval

---

### 5. EphemeralAgentManager
**Purpose:** Spawn isolated sandboxed agents for code execution, testing, and validation

**Capabilities:**
- Process isolation (namespaces, cgroups, seccomp on Linux; Job Objects on Windows)
- Resource limits (CPU, memory, disk, network, time)
- File system virtualization (overlayfs, bind mounts)
- Agent lifecycle: spawn → monitor → communicate → kill
- Stdout/stderr streaming
- Exit code and resource usage reporting
- Pre-built agent images: Python, Node.js, Rust, Go, Shell

**API Surface:**
```rust
pub struct EphemeralAgentManager {
    pub async fn spawn(&self, spec: AgentSpec) -> Result<AgentHandle>;
    pub async fn execute(&self, handle: &AgentHandle, cmd: &str) -> Result<ExecutionResult>;
    pub async fn stream_output(&self, handle: &AgentHandle) -> Result<OutputStream>;
    pub async fn kill(&self, handle: &AgentHandle) -> Result<()>;
    pub async fn get_stats(&self, handle: &AgentHandle) -> Result<AgentStats>;
}
```

**Data Structures:**
- `AgentSpec`: image, env, mounts, limits, network, working_dir, entrypoint
- `AgentHandle`: id, pid, spec, status, created_at
- `ExecutionResult`: exit_code, stdout, stderr, duration_ms, peak_memory_mb
- `AgentStats`: cpu_percent, memory_mb, disk_mb, network_rx_tx, uptime_sec

**Sandbox Profiles:**
- `minimal`: read-only fs, no net, 100MB RAM, 10s timeout
- `development`: rw workspace, localhost net, 2GB RAM, 5min timeout
- `testing`: rw temp, no net, 1GB RAM, 2min timeout
- `full`: rw workspace, full net, 4GB RAM, 10min timeout

---

## Tauri v2 Integration

### Commands (invoke from frontend)

```typescript
// FileSystem
await invoke('fs:scan_workspace', { root: string });
await invoke('fs:read_file', { path: string });
await invoke('fs:write_file', { path: string, content: string });
await invoke('fs:modify_file', { path: string, ops: ModificationOp[] });
await invoke('fs:search', { query: string });
on('fs:file_event', (event: FileEvent) => ...);

// Templates
await invoke('templates:list', { category?: string });
await invoke('templates:get', { id: string });
await invoke('templates:render', { id: string, variables: Record<string, any> });
await invoke('templates:install', { source: TemplateSource });

// Context
await invoke('context:assemble', { request: ContextRequest });

// Network
await invoke('network:discover_peers');
await invoke('network:distribute_task', { task: LLMTask });
on('network:peer_event', (event: PeerEvent) => ...);

// Agents
await invoke('agent:spawn', { spec: AgentSpec });
await invoke('agent:execute', { handle: string, command: string });
await invoke('agent:kill', { handle: string });
on('agent:output', (event: OutputEvent) => ...);
```

---

## Configuration

### `locus.toml` (Project config)
```toml
[workspace]
root = "."
ignore = [".git", "target", "node_modules", "dist"]

[templates]
paths = ["~/.locus/templates", ".locus/templates"]
auto_discover = true

[network]
enabled = true
service_name = "_locus-llm._tcp.local."
advertise_local = true
preferred_model = "llama3.1:8b"

[agents]
default_profile = "development"
profiles = {
    minimal = { memory_mb = 100, timeout_sec = 10, network = false }
    development = { memory_mb = 2048, timeout_sec = 300, network = "localhost" }
    testing = { memory_mb = 1024, timeout_sec = 120, network = false }
    full = { memory_mb = 4096, timeout_sec = 600, network = true }
}

[context]
max_tokens = 32000
compression_threshold = 0.8
include_git_history = false
```

---

## Dependencies

### Core Rust Dependencies
```toml
# Tauri v2
tauri = { version = "2", features = ["macros"] }
tauri-plugin-fs = "2"
tauri-plugin-shell = "2"
tauri-plugin-dialog = "2"
tauri-plugin-opener = "2"

# Async Runtime
tokio = { version = "1", features = ["full"] }
tokio-stream = "0.1"
tokio-util = { version = "0.7", features = ["codec"] }

# File System
notify = { version = "6", features = ["tokio"] }
walkdir = "2"
ignore = "0.12"
globset = "0.4"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
toml = "0.8"

# Templates
tera = "1"
jinja2 = "0.1"  # alternative

# Network/mDNS
mdns-sd = "0.10"
async-mdns = "0.2"
tokio-tungstenite = { version = "0.21", features = ["native-tls"] }  # WebRTC signaling

# Sandboxing
nix = "0.28"  # Linux namespaces
job-rs = "0.3"  # Windows Job Objects
docker-api = "0.12"  # optional container backend

# Utilities
anyhow = "1"
thiserror = "1"
tracing = { version = "0.1", features = ["std"] }
tracing-subscriber = { version = "0.3", features = ["env-filter", "fmt"] }
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

---

## Project Structure

```
locus/
├── Cargo.toml                 # Workspace root
├── SPEC.md                    # This file
├── locus.toml                 # Default config
├── crates/
│   ├── locus-core/            # Core types, errors, config
│   ├── locus-fs/              # FileSystemEngine
│   ├── locus-templates/       # TemplateEngine
│   ├── locus-context/         # ContextManager
│   ├── locus-network/         # NetworkOrchestrator
│   ├── locus-agents/          # EphemeralAgentManager
│   └── locus-tauri/           # Tauri commands, IPC
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       ├── main.rs
│       ├── commands/
│       │   ├── fs.rs
│       │   ├── templates.rs
│       │   ├── context.rs
│       │   ├── network.rs
│       │   └── agents.rs
│       └── state.rs
└── src/                       # Frontend (React + TypeScript)
    ├── components/
    ├── hooks/
    ├── stores/
    └── main.tsx
```

---

## Implementation Phases

### Phase 1: Foundation (Week 1)
- [ ] Cargo workspace setup
- [ ] Core types, error handling, config
- [ ] FileSystemEngine basic ops (scan, read, write)
- [ ] Tauri v2 project init

### Phase 2: File System & Templates (Week 2)
- [ ] File watching with notify
- [ ] TemplateEngine with Tera
- [ ] Built-in template library
- [ ] Search and indexing

### Phase 3: Context & Network (Week 3)
- [ ] ContextManager assembly
- [ ] Token estimation/budgeting
- [ ] mDNS discovery
- [ ] Peer communication

### Phase 4: Agents & Integration (Week 4)
- [ ] EphemeralAgentManager
- [ ] Sandbox profiles
- [ ] Tauri command layer
- [ ] Frontend integration

### Phase 5: Polish (Week 5)
- [ ] Error handling, logging
- [ ] Performance optimization
- [ ] Cross-platform testing
- [ ] Documentation

---

## Acceptance Criteria

1. **FileSystemEngine**: Scans 10k files < 2s, watches with < 100ms latency
2. **TemplateEngine**: Renders complex template < 50ms, supports 50+ built-ins
3. **ContextManager**: Assembles 32k token context < 500ms
4. **NetworkOrchestrator**: Discovers peers < 3s, distributes task < 100ms overhead
5. **EphemeralAgentManager**: Spawns agent < 500ms, kills < 100ms
6. **All local**: Zero network calls to external services unless explicitly configured
7. **Cross-platform**: Builds and runs on Windows, macOS, Linux