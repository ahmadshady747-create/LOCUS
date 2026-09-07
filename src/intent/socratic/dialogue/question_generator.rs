//! Socratic Question Generator and Intent Resolution Engine.
//!
//! Transforms high-entropy intent reports into focused, closed Socratic dialogues
//! containing exactly 3 structured architectural choices (1 Recommended + 2 Alternatives),
//! accompanied by concrete code sketches and engineering tradeoff matrices.

#![forbid(unsafe_code)]

use super::taste_engine::ArchitecturalTasteEngine;
use super::types::{
    ArchitecturalChoice, ArchitecturalPattern, IntentResolution, SocraticQuestion, TasteScore,
};
use crate::intent::socratic::types::{EdgeCaseKind, IntentEntropyReport};

/// High-throughput Socratic Question and Architectural Dialogue Generator.
#[derive(Debug, Clone)]
pub struct SocraticQuestionGenerator {
    taste_engine: ArchitecturalTasteEngine,
}

impl Default for SocraticQuestionGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SocraticQuestionGenerator {
    pub fn new() -> Self {
        Self {
            taste_engine: ArchitecturalTasteEngine::new(),
        }
    }

    /// Generates structured Socratic clarification questions from an `IntentEntropyReport`.
    ///
    /// Evaluates in sub-0.05ms, producing constrained choices classified by architectural taste.
    pub fn generate_dialogue(&self, report: &IntentEntropyReport) -> Vec<SocraticQuestion> {
        let mut questions = Vec::with_capacity(3);

        // 1. Check for Edge-Case & Error Handling Gaps
        if let Some(edge_violation) = report.underspecified_edge_cases.iter().find(|e| {
            e.kind == EdgeCaseKind::MissingErrorHandling
                || e.kind == EdgeCaseKind::NullabilityBoundaryGap
                || e.kind == EdgeCaseKind::ConcurrencyGap
        }) {
            questions.push(self.generate_error_and_resilience_question(&report.intent, edge_violation.kind));
        }

        // 2. Check for Data Layer / State Flow Ambiguity
        let intent_lower = report.intent.to_lowercase();
        let is_data_intent = intent_lower.contains("state")
            || intent_lower.contains("cache")
            || intent_lower.contains("store")
            || intent_lower.contains("data")
            || intent_lower.contains("db")
            || intent_lower.contains("repository")
            || intent_lower.contains("collection")
            || intent_lower.contains("buffer");

        if is_data_intent || questions.is_empty() {
            questions.push(self.generate_data_architecture_question(&report.intent));
        }

        // 3. Check for Inter-Module Communication / API Architecture
        let is_api_intent = intent_lower.contains("api")
            || intent_lower.contains("service")
            || intent_lower.contains("trait")
            || intent_lower.contains("client")
            || intent_lower.contains("handler")
            || intent_lower.contains("interface")
            || intent_lower.contains("channel")
            || intent_lower.contains("rpc");

        if is_api_intent || (!report.ambiguous_targets.is_empty() && questions.len() < 2) {
            questions.push(self.generate_api_communication_question(&report.intent));
        }

        // Guarantee at least 1 primary architectural question if entropy was triggered
        if questions.is_empty() && report.requires_socratic_dialogue {
            questions.push(self.generate_general_architecture_question(&report.intent));
        }

        questions
    }

    /// Generates a Socratic question for Error Handling, Concurrency & Resilience Gaps.
    pub fn generate_error_and_resilience_question(
        &self,
        prompt_context: &str,
        kind: EdgeCaseKind,
    ) -> SocraticQuestion {
        let (ambiguity_source, context_summary) = match kind {
            EdgeCaseKind::MissingErrorHandling => (
                "Error Handling & Boundary Resilience".to_string(),
                "The prompt lacks an explicit failure domain model and recovery strategy.",
            ),
            EdgeCaseKind::ConcurrencyGap => (
                "Thread Concurrency & Shared State Safety".to_string(),
                "The prompt specifies concurrent access without locking or ownership invariants.",
            ),
            EdgeCaseKind::NullabilityBoundaryGap => (
                "Nullability & Optional Value Invariants".to_string(),
                "The prompt accesses nullable/optional entities without explicit fallback handling.",
            ),
            _ => (
                "System Invariants & Guard Strategy".to_string(),
                "The prompt leaves boundary edge-case handling underspecified.",
            ),
        };

        let mut choices = vec![
            ArchitecturalChoice {
                id: "choice_err_domain_enum".to_string(),
                title: "Exhaustive Typed Domain Error with Non-Panic Result<T, E>".to_string(),
                description: "Define a zero-cost, strongly-typed domain error enum implementing std::error::Error with non-panicking Result propagation.".to_string(),
                pattern: ArchitecturalPattern::CleanArchitecture,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    9, 9, 9, 8,
                    "Maximum safety, zero runtime panics, full compiler-enforced error handling with zero allocation overhead.",
                ),
                code_sketch: Some("pub enum DomainError {\n    NotFound(String),\n    InvalidState { expected: &'static str, actual: String },\n}\npub fn execute() -> Result<Payload, DomainError>;".to_string()),
                blast_radius_impact: "Localized: Downstream callers handle structured domain variants without breaking ABI.".to_string(),
            },
            ArchitecturalChoice {
                id: "choice_err_graceful_fallback".to_string(),
                title: "Graceful Fallback with Default Strategy & Structured Warning Telemetry".to_string(),
                description: "Intercept faults and fallback to cached or default state while emitting structured diagnostic telemetry events.".to_string(),
                pattern: ArchitecturalPattern::FunctionalPipeline,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    8, 8, 7, 9,
                    "High resilience against catastrophic failure, fast development velocity, slight masking of edge conditions.",
                ),
                code_sketch: Some("pub fn execute_with_fallback() -> Payload {\n    attempt_operation().unwrap_or_else(|err| {\n        telemetry::warn!(\"Fallback triggered: {}\", err);\n        Payload::default()\n    })\n}".to_string()),
                blast_radius_impact: "Minimal: Preserves caller semantics by providing guaranteed non-null responses.".to_string(),
            },
            ArchitecturalChoice {
                id: "choice_err_fail_fast".to_string(),
                title: "Strict Fail-Fast Isolation with Subsystem Boundary Panic Guard".to_string(),
                description: "Immediately abort corrupt operations at the task boundary with catch_unwind isolation to maintain system-wide swarm integrity.".to_string(),
                pattern: ArchitecturalPattern::ActorMesh,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    9, 7, 6, 6,
                    "Strict safety boundaries, prevents state corruption, requires supervisor restart mechanisms.",
                ),
                code_sketch: Some("pub fn isolated_execute() -> Result<(), PanicFault> {\n    std::panic::catch_unwind(|| {\n        critical_inner_work();\n    })\n}".to_string()),
                blast_radius_impact: "Subsystem-level: Faults are contained strictly within the executing worker actor.".to_string(),
            },
        ];

        self.taste_engine.rank_and_designate_recommendation(&mut choices);
        let recommended_id = choices.first().map(|c| c.id.clone()).unwrap_or_default();

        SocraticQuestion {
            id: format!("sq_resilience_{}", kind),
            prompt_context: format!("{}: {}", context_summary, prompt_context),
            ambiguity_source,
            choices,
            recommended_choice_id: recommended_id,
        }
    }

    /// Generates a Socratic question for Data Layer, Storage, and State Lifecycle.
    pub fn generate_data_architecture_question(&self, prompt_context: &str) -> SocraticQuestion {
        let mut choices = vec![
            ArchitecturalChoice {
                id: "choice_data_clean_repo".to_string(),
                title: "Hexagonal Repository Pattern with Decoupled Trait Storage Driver".to_string(),
                description: "Decouple domain business logic from physical storage engines via sovereign Rust traits and immutable snapshots.".to_string(),
                pattern: ArchitecturalPattern::Hexagonal,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    8, 8, 10, 8,
                    "100% isolable mock testing, hot-swappable storage backends, clean architectural boundaries.",
                ),
                code_sketch: Some("pub trait DataRepository: Send + Sync {\n    fn get_by_id(&self, id: u64) -> Option<Entity>;\n    fn save(&self, entity: Entity) -> Result<(), RepoError>;\n}".to_string()),
                blast_radius_impact: "Zero blast radius: Storage changes never leak into core business domain entities.".to_string(),
            },
            ArchitecturalChoice {
                id: "choice_data_zero_alloc_dod".to_string(),
                title: "Data-Oriented Zero-Allocation Buffer with Cache-Locality Layout".to_string(),
                description: "Utilize contiguous Struct-of-Arrays (SoA) memory arena buffers for sub-microsecond vector cache efficiency.".to_string(),
                pattern: ArchitecturalPattern::DataOriented,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    10, 10, 7, 6,
                    "Sub-microsecond throughput, 0 heap churn, requires fixed-capacity memory arena planning.",
                ),
                code_sketch: Some("pub struct MemoryArena<T> {\n    buffer: Vec<T>,\n    capacity: usize,\n}\nimpl<T> MemoryArena<T> {\n    pub fn allocate(&mut self, item: T) -> SlotId;\n}".to_string()),
                blast_radius_impact: "Isolated: Internal high-performance arena contained within the data engine.".to_string(),
            },
            ArchitecturalChoice {
                id: "choice_data_actor_state".to_string(),
                title: "Actor-Mesh Shared-Nothing State with Non-Blocking Mailbox Queue".to_string(),
                description: "Encapsulate mutable state completely within a dedicated worker actor responding to asynchronous immutable messages.".to_string(),
                pattern: ArchitecturalPattern::ActorMesh,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    8, 7, 8, 7,
                    "Zero lock contention, deadlock immunity, asynchronous message passing overhead.",
                ),
                code_sketch: Some("pub struct StateActor {\n    receiver: flume::Receiver<Command>,\n    state: LocalState,\n}".to_string()),
                blast_radius_impact: "Isolated: Concurrency confined to actor message loop.".to_string(),
            },
        ];

        self.taste_engine.rank_and_designate_recommendation(&mut choices);
        let recommended_id = choices.first().map(|c| c.id.clone()).unwrap_or_default();

        SocraticQuestion {
            id: "sq_data_storage_layout".to_string(),
            prompt_context: format!("Data architecture & state flow strategy for: {}", prompt_context),
            ambiguity_source: "Data Storage & State Mutation Strategy".to_string(),
            choices,
            recommended_choice_id: recommended_id,
        }
    }

    /// Generates a Socratic question for Inter-Module Communication, APIs, and Trait Boundaries.
    pub fn generate_api_communication_question(&self, prompt_context: &str) -> SocraticQuestion {
        let mut choices = vec![
            ArchitecturalChoice {
                id: "choice_api_ports_adapters".to_string(),
                title: "Clean Domain Service with Inversion-of-Control Ports & Adapters".to_string(),
                description: "Structure cross-module service interactions through decoupled interface traits injected at construction time.".to_string(),
                pattern: ArchitecturalPattern::CleanArchitecture,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    8, 8, 10, 8,
                    "Flawless unit testing, zero coupling between subsystems, clear contract boundaries.",
                ),
                code_sketch: Some("pub struct Service<D: Driver> {\n    driver: Arc<D>,\n}\nimpl<D: Driver> Service<D> {\n    pub async fn process(&self, req: Request) -> Response;\n}".to_string()),
                blast_radius_impact: "Extremely low: Callers interact only with the abstract trait contract.".to_string(),
            },
            ArchitecturalChoice {
                id: "choice_api_functional_pipeline".to_string(),
                title: "Composable Pure Functional Pipeline with Middlewares".to_string(),
                description: "Model API workflows as a deterministic sequence of immutable transformation functions and filter middleware stages.".to_string(),
                pattern: ArchitecturalPattern::FunctionalPipeline,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    9, 9, 8, 8,
                    "High reusability of individual pipeline filters, easy verification, strictly linear flow.",
                ),
                code_sketch: Some("pub fn pipeline(ctx: Context) -> Result<Context, Error> {\n    validate(ctx).and_then(enrich).and_then(commit)\n}".to_string()),
                blast_radius_impact: "Minimal: Adding pipeline stages does not modify existing stages.".to_string(),
            },
            ArchitecturalChoice {
                id: "choice_api_microkernel_plugin".to_string(),
                title: "Sovereign Microkernel Plugin Architecture with Dynamic Dispatch".to_string(),
                description: "Establish a minimal core kernel with dynamic registration of handler extensions via capability token leases.".to_string(),
                pattern: ArchitecturalPattern::MicroKernel,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    8, 7, 9, 7,
                    "Infinite extensibility, dynamic hot-reloading capability, requires plugin registry manager.",
                ),
                code_sketch: Some("pub struct Kernel {\n    handlers: HashMap<CapabilityId, Box<dyn Plugin>>,\n}".to_string()),
                blast_radius_impact: "Contained: Core kernel remains frozen and immune to extension regressions.".to_string(),
            },
        ];

        self.taste_engine.rank_and_designate_recommendation(&mut choices);
        let recommended_id = choices.first().map(|c| c.id.clone()).unwrap_or_default();

        SocraticQuestion {
            id: "sq_api_communication_pattern".to_string(),
            prompt_context: format!("Inter-module API & trait boundaries for: {}", prompt_context),
            ambiguity_source: "Communication Protocol & API Architecture".to_string(),
            choices,
            recommended_choice_id: recommended_id,
        }
    }

    /// Fallback general architectural question.
    pub fn generate_general_architecture_question(&self, prompt_context: &str) -> SocraticQuestion {
        let mut choices = vec![
            ArchitecturalChoice {
                id: "choice_gen_clean_arch".to_string(),
                title: "Modular Clean Architecture with Decoupled Domain Logic".to_string(),
                description: "Separate domain entities, use cases, and infrastructure adapters into strict layered boundaries.".to_string(),
                pattern: ArchitecturalPattern::CleanArchitecture,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    8, 8, 10, 8,
                    "High maintainability, clear structural boundaries, testable in pure isolation.",
                ),
                code_sketch: Some("pub mod domain;\npub mod usecase;\npub mod adapter;\n".to_string()),
                blast_radius_impact: "Localized: Changes strictly constrained within individual layers.".to_string(),
            },
            ArchitecturalChoice {
                id: "choice_gen_zero_alloc".to_string(),
                title: "Zero-Allocation Direct Data Pipeline".to_string(),
                description: "Implement high-throughput linear transformation pipeline with stack-allocated buffers and zero GC overhead.".to_string(),
                pattern: ArchitecturalPattern::DataOriented,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    10, 10, 7, 7,
                    "Ultra-high performance, sub-microsecond latency, 0 allocation overhead.",
                ),
                code_sketch: Some("pub fn process_stream(input: &[u8], output: &mut [u8]) -> usize;\n".to_string()),
                blast_radius_impact: "Localized: Direct stream processing without shared global state.".to_string(),
            },
            ArchitecturalChoice {
                id: "choice_gen_functional".to_string(),
                title: "Pure Functional State-Machine Transformation".to_string(),
                description: "Model system state transitions as an explicit finite state machine with immutable transition functions.".to_string(),
                pattern: ArchitecturalPattern::FunctionalPipeline,
                taste_score: TasteScore::ZERO,
                is_recommended: false,
                tradeoffs: ArchitecturalTasteEngine::create_tradeoff_matrix(
                    9, 9, 8, 8,
                    "Deterministic state space, mathematically provable transition safety, zero deadlocks.",
                ),
                code_sketch: Some("pub fn step_state(state: State, event: Event) -> State;\n".to_string()),
                blast_radius_impact: "Zero: Immutability prevents state leakage across components.".to_string(),
            },
        ];

        self.taste_engine.rank_and_designate_recommendation(&mut choices);
        let recommended_id = choices.first().map(|c| c.id.clone()).unwrap_or_default();

        SocraticQuestion {
            id: "sq_general_architecture".to_string(),
            prompt_context: format!("Structural architecture strategy for: {}", prompt_context),
            ambiguity_source: "High-Level System & Component Architecture".to_string(),
            choices,
            recommended_choice_id: recommended_id,
        }
    }

    /// Resolves a user selection on a Socratic question into an actionable `IntentResolution`.
    pub fn resolve_choice(
        &self,
        question: &SocraticQuestion,
        chosen_choice_id: &str,
    ) -> Option<IntentResolution> {
        let choice = question.get_choice(chosen_choice_id)?;

        let mut contract_hints = Vec::new();
        let mut enforced_invariants = Vec::new();

        match choice.pattern {
            ArchitecturalPattern::CleanArchitecture => {
                contract_hints.push("Enforce decoupled domain use-case boundary".to_string());
                contract_hints.push("Return Result<T, DomainError> with non-panicking variants".to_string());
                enforced_invariants.push("Zero panic unwrap on domain operations".to_string());
                enforced_invariants.push("Domain entities must not depend on concrete IO drivers".to_string());
            }
            ArchitecturalPattern::Hexagonal => {
                contract_hints.push("Expose storage and driver traits in ports definition".to_string());
                contract_hints.push("Inject driver dependencies via Arc<dyn Trait>".to_string());
                enforced_invariants.push("100% trait decoupling for external infrastructure".to_string());
                enforced_invariants.push("Deterministic delimiter balance across port adapters".to_string());
            }
            ArchitecturalPattern::DataOriented => {
                contract_hints.push("Use contiguous slice buffers &[T] and &mut [T]".to_string());
                contract_hints.push("Zero heap allocation in hot processing loops".to_string());
                enforced_invariants.push("All buffer slices must be bounds-checked without panic".to_string());
                enforced_invariants.push("Zero memory leaks on arena deallocation".to_string());
            }
            ArchitecturalPattern::ActorMesh => {
                contract_hints.push("Encapsulate state inside dedicated worker channel loop".to_string());
                contract_hints.push("Communicate exclusively via immutable message structs".to_string());
                enforced_invariants.push("Never hold mutex locks across await boundaries".to_string());
                enforced_invariants.push("Bounded message channels to prevent queue deadlocks".to_string());
            }
            ArchitecturalPattern::FunctionalPipeline => {
                contract_hints.push("Compose pure functions (A -> Result<B, E>)".to_string());
                contract_hints.push("Immutable input context parameters".to_string());
                enforced_invariants.push("Zero side-effects in pure pipeline stages".to_string());
                enforced_invariants.push("Total exhaustive pattern matching on intermediate states".to_string());
            }
            ArchitecturalPattern::MicroKernel => {
                contract_hints.push("Core kernel registers capability plugins via token leases".to_string());
                contract_hints.push("Dynamic capability dispatch registry".to_string());
                enforced_invariants.push("Plugin failures must not crash the core microkernel".to_string());
                enforced_invariants.push("Strict capability lease verification on each invocation".to_string());
            }
        }

        Some(IntentResolution {
            question_id: question.id.clone(),
            selected_choice_id: chosen_choice_id.to_string(),
            applied_pattern: choice.pattern,
            synthesized_contract_hints: contract_hints,
            enforced_invariants,
            final_taste_score: choice.taste_score,
        })
    }
}
