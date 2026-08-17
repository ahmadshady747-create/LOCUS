import type {
  QaReport,
  SpecAlignmentReport,
  TaskGraph,
} from "../types";

export interface PlaygroundData {
  goal: string;
  graph: TaskGraph;
  qaReports: Record<string, QaReport>;
  specReport: SpecAlignmentReport;
}

export function generatePlaygroundDemo(): PlaygroundData {
  const goal = "Add high-performance Redis caching layer with token validation & adversarial unit tests";

  const graph: TaskGraph = {
    id: "playground-graph-001",
    goal,
    status: "in_progress",
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    nodes: [
      {
        id: "step-1",
        title: "Extract Symbolic Constraints & Interface Signatures",
        description: "Analyze cache key structures and hashing functions in locus-core.",
        node_type: "analysis",
        dependencies: [],
        status: "completed",
        payload: {
          target_file: "crates/locus-core/src/types.rs",
        },
        result: {
          success: true,
          output: "✓ Extracted 14 symbolic constraints from crates/locus-core/src/types.rs in 12ms",
          duration_ms: 12,
        },
        auto_execute: true,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
      {
        id: "step-2",
        title: "Implement Async Cache Storage Engine",
        description: "Write atomic get/set operations with TTL expiration and connection pooling.",
        node_type: "code_edit",
        dependencies: ["step-1"],
        status: "completed",
        payload: {
          target_file: "crates/locus-fs/src/cache_store.rs",
          search_replace_block: `<<<<<<< SEARCH
// Cache placeholder
=======
pub async fn get_cached_item(&self, key: &str) -> Result<Option<Vec<u8>>> {
    let hashed = blake3::hash(key.as_bytes());
    self.pool.get(hashed.as_bytes()).await
}
>>>>>>> REPLACE`,
        },
        result: {
          success: true,
          output: "✓ Applied Search & Replace hunk cleanly (1 file modified, 0 rejects)",
          duration_ms: 45,
        },
        auto_execute: true,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
      {
        id: "step-3",
        title: "Run Adversarial QA Audit & Fuzz Simulation",
        description: "Verify that deadlocks across .await and unwrap panics are eliminated.",
        node_type: "test",
        dependencies: ["step-2"],
        status: "ready",
        payload: {
          target_file: "crates/locus-fs/src/cache_store.rs",
          shell_command: "cargo test -p locus-fs",
        },
        auto_execute: true,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      },
    ],
  };

  const qaReports: Record<string, QaReport> = {
    "step-2": {
      score: 98,
      is_approved: true,
      summary: "High robustness score. No synchronous locks held across .await boundaries. Validated against 5 boundary fuzz inputs.",
      risks: [],
      fuzz_cases: [
        {
          input_name: "Null / Empty Key Buffer",
          input_value: "''",
          expected_behavior: "Returns Ok(None) without panicking",
        },
        {
          input_name: "Large Binary Payload (16MB)",
          input_value: "0xFF * 16777216",
          expected_behavior: "Gracefully rejected by max buffer constraint",
        },
        {
          input_name: "Malformed UTF-8 Sequence",
          input_value: "\\xF0\\x28\\x8C\\xBC",
          expected_behavior: "Safely sanitized and hashed with Blake3",
        },
      ],
    },
  };

  const specReport: SpecAlignmentReport = {
    goal,
    has_ambiguity: true,
    ambiguities: [
      {
        id: "amb-cache-persistence",
        category: "persistence",
        question: "Select Redis Persistence & Cache Invalidation Strategy:",
        selected_option_id: "opt-inmem-ttl",
        options: [
          {
            id: "opt-inmem-ttl",
            title: "In-Memory LRU with TTL Expiration",
            description: "High throughput, zero disk overhead, automatic eviction on memory pressure.",
            pros: ["Sub-millisecond latency", "Zero disk I/O cost"],
            cons: ["Cache cleared on app restart"],
            recommended: true,
          },
          {
            id: "opt-atomic-json",
            title: "Atomic Disk-Backed Snapshot Cache",
            description: "Persists warm cache across app restarts into .locus/cache.json.",
            pros: ["Preserves cache across restarts", "Instant cold boot"],
            cons: ["Slightly higher write latency"],
            recommended: false,
          },
        ],
      },
    ],
    aligned_constraints: [
      "Use in-memory LRU cache with configurable TTL expiration",
    ],
  };

  return { goal, graph, qaReports, specReport };
}
