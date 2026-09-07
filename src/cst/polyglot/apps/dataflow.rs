//! Application & Enterprise State & Dataflow Tracker.
//!
//! Analyzes state lifecycle, reactive hooks (`useState`, `useReducer`, `Signals`),
//! Flutter state management (`setState`, `ChangeNotifier`), Java concurrency primitives
//! (`AtomicReference`, `ConcurrentHashMap`), and detects asynchronous data race hazards.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use crate::cst::green::SyntaxKind;
use crate::cst::lexer::tokenize;
use crate::types::Language;

/// Classification of a state source declaration.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StateSourceKind {
    // React / Modern Web
    ReactUseState,
    ReactUseReducer,
    ReactUseRef,
    PreactSignal,
    SvelteRuneState,
    VueRefOrReactive,

    // Flutter / Mobile
    FlutterSetState,
    FlutterChangeNotifier,
    FlutterValueNotifier,
    FlutterBlocOrCubit,

    // Enterprise / Backend
    JavaAtomicPrimitive,
    JavaConcurrentCollection,
    PythonSessionState,
    PhpSessionState,
}

/// A state declaration source tracked in the component/module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateSource {
    pub name: String,
    pub setter_name: Option<String>,
    pub kind: StateSourceKind,
    pub initial_value: Option<String>,
    pub byte_offset: u32,
    pub is_concurrent_safe: bool,
}

/// A state mutation site modifying a tracked state variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateMutation {
    pub state_name: String,
    pub mutator: String,
    pub is_functional_update: bool,
    pub is_in_async_context: bool,
    pub byte_offset: u32,
}

/// A state read site consuming the state variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRead {
    pub state_name: String,
    pub is_in_render_path: bool,
    pub is_in_async_context: bool,
    pub byte_offset: u32,
}

/// A detected state hazard (e.g. async race condition or unhandled async write).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataflowHazard {
    pub state_name: String,
    pub hazard_type: String,
    pub message: String,
    pub byte_offset: u32,
}

/// Directed Dataflow & State Dependency Graph.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateDataflowGraph {
    pub sources: Vec<StateSource>,
    pub mutations: Vec<StateMutation>,
    pub reads: Vec<StateRead>,
    pub hazards: Vec<DataflowHazard>,
}

/// Comprehensive Dataflow & State Analysis Report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataflowReport {
    pub language: Language,
    pub total_state_variables: usize,
    pub total_mutations: usize,
    pub total_reads: usize,
    pub hazard_count: usize,
    pub is_safe: bool,
    pub graph: StateDataflowGraph,
}

/// High-throughput State & Dataflow Tracker.
#[derive(Debug, Default, Clone)]
pub struct DataflowTracker;

impl DataflowTracker {
    pub fn new() -> Self {
        Self
    }

    /// Performs deep state and dataflow hazard analysis on source text.
    pub fn analyze(&self, source: &str, language: Language) -> DataflowReport {
        let mut graph = StateDataflowGraph::default();

        match language {
            Language::TypeScript | Language::JavaScript | Language::Tsx | Language::Jsx | Language::Svelte | Language::Astro | Language::Vue => {
                self.analyze_react_and_web_state(source, &mut graph);
            }
            Language::Dart => {
                self.analyze_flutter_state(source, &mut graph);
            }
            Language::Java => {
                self.analyze_java_state(source, &mut graph);
            }
            Language::Python => {
                self.analyze_python_state(source, &mut graph);
            }
            Language::Php => {
                self.analyze_php_state(source, &mut graph);
            }
            _ => {}
        }

        let hazard_count = graph.hazards.len();
        DataflowReport {
            language,
            total_state_variables: graph.sources.len(),
            total_mutations: graph.mutations.len(),
            total_reads: graph.reads.len(),
            hazard_count,
            is_safe: hazard_count == 0,
            graph,
        }
    }

    fn analyze_react_and_web_state(&self, source: &str, graph: &mut StateDataflowGraph) {
        let tokens = tokenize(source);
        let len = tokens.len();

        let mut byte_offset = 0;
        let mut token_offsets = Vec::with_capacity(len);
        for &(_, text) in &tokens {
            token_offsets.push(byte_offset);
            byte_offset += text.len();
        }

        let mut i = 0;
        let mut in_async_scope = false;

        while i < len {
            let (kind, text) = tokens[i];

            if kind == SyntaxKind::AsyncKw {
                in_async_scope = true;
            }

            // 1. Detect React useState: `const [count, setCount] = useState(0);`
            if kind == SyntaxKind::ConstKw || kind == SyntaxKind::LetKw {
                let mut scan = i + 1;
                while scan < len && tokens[scan].0.is_trivia() {
                    scan += 1;
                }
                if scan < len && tokens[scan].0 == SyntaxKind::OpenBracket {
                    let mut var_idx = scan + 1;
                    while var_idx < len && tokens[var_idx].0.is_trivia() {
                        var_idx += 1;
                    }
                    if var_idx < len && tokens[var_idx].0 == SyntaxKind::Ident {
                        let state_name = tokens[var_idx].1.to_string();
                        let mut setter_name = None;

                        let mut comma_idx = var_idx + 1;
                        while comma_idx < len && tokens[comma_idx].0.is_trivia() {
                            comma_idx += 1;
                        }
                        if comma_idx < len && tokens[comma_idx].0 == SyntaxKind::Comma {
                            let mut set_idx = comma_idx + 1;
                            while set_idx < len && tokens[set_idx].0.is_trivia() {
                                set_idx += 1;
                            }
                            if set_idx < len && tokens[set_idx].0 == SyntaxKind::Ident {
                                setter_name = Some(tokens[set_idx].1.to_string());
                            }
                        }

                        // Check if assigned from useState(...)
                        let mut after_bracket = var_idx + 1;
                        while after_bracket < len && tokens[after_bracket].0 != SyntaxKind::CloseBracket {
                            after_bracket += 1;
                        }
                        if after_bracket < len {
                            let mut eq_idx = after_bracket + 1;
                            while eq_idx < len && tokens[eq_idx].0.is_trivia() {
                                eq_idx += 1;
                            }
                            if eq_idx < len && tokens[eq_idx].0 == SyntaxKind::Eq {
                                let mut call_idx = eq_idx + 1;
                                while call_idx < len && tokens[call_idx].0.is_trivia() {
                                    call_idx += 1;
                                }
                                if call_idx < len && tokens[call_idx].0 == SyntaxKind::Ident {
                                    let hook_name = tokens[call_idx].1;
                                    let kind = match hook_name {
                                        "useState" => Some(StateSourceKind::ReactUseState),
                                        "useReducer" => Some(StateSourceKind::ReactUseReducer),
                                        "useRef" => Some(StateSourceKind::ReactUseRef),
                                        "signal" | "useSignal" => Some(StateSourceKind::PreactSignal),
                                        _ => None,
                                    };

                                    if let Some(state_kind) = kind {
                                        graph.sources.push(StateSource {
                                            name: state_name,
                                            setter_name,
                                            kind: state_kind,
                                            initial_value: None,
                                            byte_offset: token_offsets[i] as u32,
                                            is_concurrent_safe: false,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 2. Detect SetState / Mutator Invocation and check for async hazards
            if kind == SyntaxKind::Ident {
                let call_name = text;
                for src in &graph.sources {
                    if let Some(setter) = &src.setter_name {
                        if setter == call_name {
                            let mut open_p = i + 1;
                            while open_p < len && tokens[open_p].0.is_trivia() {
                                open_p += 1;
                            }
                            if open_p < len && tokens[open_p].0 == SyntaxKind::OpenParen {
                                let mut arg_scan = open_p + 1;
                                let mut is_functional = false;
                                while arg_scan < len && tokens[arg_scan].0 != SyntaxKind::CloseParen && arg_scan < open_p + 15 {
                                    if tokens[arg_scan].0 == SyntaxKind::FatArrow || tokens[arg_scan].0 == SyntaxKind::FnKw {
                                        is_functional = true;
                                        break;
                                    }
                                    arg_scan += 1;
                                }

                                let byte_off = token_offsets[i] as u32;
                                graph.mutations.push(StateMutation {
                                    state_name: src.name.clone(),
                                    mutator: call_name.to_string(),
                                    is_functional_update: is_functional,
                                    is_in_async_context: in_async_scope,
                                    byte_offset: byte_off,
                                });

                                // Race Hazard: Non-functional state update in async context
                                if in_async_scope && !is_functional {
                                    graph.hazards.push(DataflowHazard {
                                        state_name: src.name.clone(),
                                        hazard_type: "ASYNC_NON_FUNCTIONAL_STATE_UPDATE".to_string(),
                                        message: format!(
                                            "Asynchronous invocation of setter '{}' for state '{}' uses non-functional payload. Use functional update (e.g. {} => ...) to prevent lost updates.",
                                             call_name, src.name, src.name
                                        ),
                                        byte_offset: byte_off,
                                    });
                                }
                            }
                        }
                    }
                }
            }

            i += 1;
        }
    }

    fn analyze_flutter_state(&self, source: &str, graph: &mut StateDataflowGraph) {
        let tokens = tokenize(source);
        let len = tokens.len();

        let mut byte_offset = 0;
        let mut token_offsets = Vec::with_capacity(len);
        for &(_, text) in &tokens {
            token_offsets.push(byte_offset);
            byte_offset += text.len();
        }

        let mut i = 0;
        let mut in_async = false;

        while i < len {
            let (kind, text) = tokens[i];

            if kind == SyntaxKind::AsyncKw {
                in_async = true;
            }

            // 1. Detect ChangeNotifier / ValueNotifier
            if kind == SyntaxKind::Ident && (text == "ChangeNotifier" || text == "ValueNotifier") {
                graph.sources.push(StateSource {
                    name: "state".to_string(),
                    setter_name: Some("notifyListeners".to_string()),
                    kind: StateSourceKind::FlutterChangeNotifier,
                    initial_value: None,
                    byte_offset: token_offsets[i] as u32,
                    is_concurrent_safe: false,
                });
            }

            // 2. Detect setState(...)
            if kind == SyntaxKind::Ident && text == "setState" {
                graph.mutations.push(StateMutation {
                    state_name: "widgetState".to_string(),
                    mutator: "setState".to_string(),
                    is_functional_update: true,
                    is_in_async_context: in_async,
                    byte_offset: token_offsets[i] as u32,
                });
            }

            i += 1;
        }
    }

    fn analyze_java_state(&self, source: &str, graph: &mut StateDataflowGraph) {
        let tokens = tokenize(source);
        let len = tokens.len();

        let mut byte_offset = 0;
        let mut token_offsets = Vec::with_capacity(len);
        for &(_, text) in &tokens {
            token_offsets.push(byte_offset);
            byte_offset += text.len();
        }

        let mut i = 0;
        while i < len {
            let (kind, text) = tokens[i];

            if kind == SyntaxKind::Ident && (text == "AtomicReference" || text == "AtomicInteger" || text == "AtomicLong" || text == "AtomicBoolean") {
                let mut scan = i + 1;
                let mut depth = 0;
                while scan < len {
                    if tokens[scan].0 == SyntaxKind::Lt {
                        depth += 1;
                    } else if tokens[scan].0 == SyntaxKind::Gt {
                        if depth > 0 {
                            depth -= 1;
                        }
                    } else if depth == 0 && tokens[scan].0 == SyntaxKind::Ident {
                        let var_name = tokens[scan].1.to_string();
                        graph.sources.push(StateSource {
                            name: var_name,
                            setter_name: Some("set".to_string()),
                            kind: StateSourceKind::JavaAtomicPrimitive,
                            initial_value: None,
                            byte_offset: token_offsets[i] as u32,
                            is_concurrent_safe: true,
                        });
                        break;
                    } else if depth == 0 && (tokens[scan].0 == SyntaxKind::Semicolon || tokens[scan].0 == SyntaxKind::Eq) {
                        break;
                    }
                    scan += 1;
                }
            } else if kind == SyntaxKind::Ident && (text == "ConcurrentHashMap" || text == "ConcurrentLinkedQueue") {
                let mut scan = i + 1;
                let mut depth = 0;
                while scan < len {
                    if tokens[scan].0 == SyntaxKind::Lt {
                        depth += 1;
                    } else if tokens[scan].0 == SyntaxKind::Gt {
                        if depth > 0 {
                            depth -= 1;
                        }
                    } else if depth == 0 && tokens[scan].0 == SyntaxKind::Ident {
                        let var_name = tokens[scan].1.to_string();
                        graph.sources.push(StateSource {
                            name: var_name,
                            setter_name: Some("put".to_string()),
                            kind: StateSourceKind::JavaConcurrentCollection,
                            initial_value: None,
                            byte_offset: token_offsets[i] as u32,
                            is_concurrent_safe: true,
                        });
                        break;
                    } else if depth == 0 && (tokens[scan].0 == SyntaxKind::Semicolon || tokens[scan].0 == SyntaxKind::Eq) {
                        break;
                    }
                    scan += 1;
                }
            } else if kind == SyntaxKind::Ident && (text == "set" || text == "compareAndSet") {
                graph.mutations.push(StateMutation {
                    state_name: "atomicState".to_string(),
                    mutator: text.to_string(),
                    is_functional_update: true,
                    is_in_async_context: false,
                    byte_offset: token_offsets[i] as u32,
                });
            }

            i += 1;
        }
    }

    fn analyze_python_state(&self, source: &str, graph: &mut StateDataflowGraph) {
        if source.contains("session_state") {
            graph.sources.push(StateSource {
                name: "session_state".to_string(),
                setter_name: None,
                kind: StateSourceKind::PythonSessionState,
                initial_value: None,
                byte_offset: 0,
                is_concurrent_safe: false,
            });
        }
    }

    fn analyze_php_state(&self, source: &str, graph: &mut StateDataflowGraph) {
        if source.contains("$_SESSION") {
            graph.sources.push(StateSource {
                name: "$_SESSION".to_string(),
                setter_name: None,
                kind: StateSourceKind::PhpSessionState,
                initial_value: None,
                byte_offset: 0,
                is_concurrent_safe: false,
            });
        }
    }
}
