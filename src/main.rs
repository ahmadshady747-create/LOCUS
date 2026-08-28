//! locus CLI — High-speed deterministic verification and AST semantic tooling.

#![forbid(unsafe_code)]

use std::env;
use std::fs;
use std::path::Path;
use std::process;
use std::time::Instant;

use locus_engine::{
    run_stdio_server, AstDiffEngine, AstGuard, AutoFixer, ContextSlicer, ContractSynthesizer,
    DataFlowTracker, HybridMatcher, Language, LeaseRegistry, NullPropagationTracker, SymbolGraph,
};

fn print_usage() {
    eprintln!(
        r#"locus-engine CLI v1.6.0

USAGE:
    locus check <file_path>
    locus skeleton <file_path>
    locus contract <intent> [--lang <lang>] [--target <file_path>]
    locus slice <symbol_name> <file_path> [--depth <depth>]
    locus graph <directory_path>
    locus impact <symbol_name> <file_or_dir> [--depth <depth>]
    locus refs <symbol_name> <directory_path>
    locus patch <file_path> --symbol <symbol_name> --with <new_code>
    locus fix <file_path>
    locus search <query> [<directory_path>]
    locus taint <file_path> [<symbol_name>]
    locus lease <acquire|release|list> ...
    locus mcp

COMMANDS:
    check       Run deterministic safety verification on a target source file
    skeleton    Extract high-level AST skeleton preserving imports, types, and component signatures
    contract    Synthesize strict type scaffolding and safety invariant checklist from intent
    slice       Extract high-density intent context slice around a target symbol
    graph       Index directory, construct cross-file symbol graph, and report architectural health
    impact      Analyze blast-radius impact and breaking change risk of modifying a symbol
    refs        Find all inbound references, imports, and call sites of a symbol across the project
    patch       Surgically replace a named AST symbol with new code
    fix         Deterministically remediate unclosed JSX, null access, and conditional hooks
    search      Execute sub-millisecond in-memory hybrid AST lexical + HNSW vector search
    taint       Trace cross-file taint flows, unvalidated inputs, and unhandled Option/null returns
    lease       Manage multi-agent concurrency leases on fully qualified symbols (FQN)
    mcp         Start MCP server over stdio for Claude Code, Cursor, and Antigravity
"#
    );
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "--version" | "-v" | "-V" | "version" => {
            println!("locus-engine v1.6.0");
        }

        "mcp" => {
            if let Err(e) = run_stdio_server() {
                eprintln!("MCP Server error: {}", e);
                process::exit(1);
            }
        }

        "contract" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: locus contract <intent> [--lang <lang>] [--target <target_path>]"
                );
                process::exit(1);
            }
            let intent = &args[2];
            let mut lang = Language::Rust;
            let mut target_path: Option<&str> = None;

            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--lang" if i + 1 < args.len() => {
                        lang = Language::from_extension(&args[i + 1]);
                        i += 2;
                    }
                    "--target" if i + 1 < args.len() => {
                        target_path = Some(&args[i + 1]);
                        i += 2;
                    }
                    _ => i += 1,
                }
            }

            let contract = ContractSynthesizer::synthesize(intent, target_path, None, lang);
            println!("{}", contract.type_scaffolding);
            println!("// Invariant Checklist:");
            for inv in contract.invariant_checklist {
                println!("// - {}", inv);
            }
        }

        "slice" => {
            if args.len() < 4 {
                eprintln!("Usage: locus slice <symbol_name> <file_path> [--depth <depth>]");
                process::exit(1);
            }
            let symbol = &args[2];
            let file_path = &args[3];
            let mut depth = 2usize;
            if args.len() > 5 && args[4] == "--depth" {
                depth = args[5].parse().unwrap_or(2);
            }

            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading '{}': {}", file_path, e);
                    process::exit(1);
                }
            };
            let ext = Path::new(file_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let lang = Language::from_extension(ext);
            let slice = ContextSlicer::slice_from_source(&content, symbol, depth, lang);
            println!("{}", slice.sliced_code);
        }

        "skeleton" => {
            if args.len() < 3 {
                eprintln!("Error: Missing <file_path> for 'skeleton' command.");
                process::exit(1);
            }
            let file_path = &args[2];
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading '{}': {}", file_path, e);
                    process::exit(1);
                }
            };
            let ext = Path::new(file_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let lang = Language::from_extension(ext);
            let skeleton = AstDiffEngine::skeletonize(&content, lang);
            println!("{}", skeleton);
        }

        "check" => {
            if args.len() < 3 {
                eprintln!("Error: Missing <file_path> for 'check' command.");
                process::exit(1);
            }
            let file_path = &args[2];
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading '{}': {}", file_path, e);
                    process::exit(1);
                }
            };

            let report = AstGuard::verify(&content);
            if report.passed {
                println!(
                    "PASS: '{}' verified safe across 20 invariant rules ({:.2}ms)",
                    file_path, report.latency_ms
                );
            } else {
                eprintln!(
                    "FAIL: '{}' violated safety invariants ({:.2}ms):",
                    file_path, report.latency_ms
                );
                for v in report.violations {
                    eprintln!("  - {}", v);
                }
                process::exit(1);
            }
        }

        "fix" => {
            if args.len() < 3 {
                eprintln!("Error: Missing <file_path> for 'fix' command.");
                process::exit(1);
            }
            let file_path = &args[2];
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading '{}': {}", file_path, e);
                    process::exit(1);
                }
            };

            let res = AutoFixer::remediate(&content);
            if res.success {
                if let Err(e) = fs::write(file_path, &res.remediated_code) {
                    eprintln!("Error writing fixed code to '{}': {}", file_path, e);
                    process::exit(1);
                }
                println!(
                    "Fixed '{}' ({} edits applied, verified safe in {:.2}ms)",
                    file_path,
                    res.edits_applied.len(),
                    res.latency_ms
                );
            } else {
                println!("No automatic fixes applied (or manual intervention required).");
            }
        }

        "graph" => {
            let path = if args.len() > 2 { &args[2] } else { "." };
            let start = Instant::now();
            let graph = SymbolGraph::index_directory(path);
            let elapsed = start.elapsed();
            let cycles = graph.detect_import_cycles();

            println!(
                "Indexed directory: '{}' ({:.2}ms)",
                path,
                elapsed.as_secs_f64() * 1000.0
            );
            println!(
                "Total files: {}, Symbols: {}, Dependency edges: {}",
                graph.file_to_symbols.len(),
                graph.nodes.len(),
                graph.edges.len()
            );
            if !cycles.is_empty() {
                println!("WARNING: Circular dependencies detected:");
                for cycle in cycles {
                    println!("  - {}", cycle.join(" -> "));
                }
            }
        }

        "impact" => {
            if args.len() < 3 {
                eprintln!("Usage: locus impact <symbol_name> [<file_or_dir>] [--depth <depth>]");
                process::exit(1);
            }
            let symbol = &args[2];
            let path = if args.len() > 3 && !args[3].starts_with("--") {
                &args[3]
            } else {
                "."
            };
            let mut depth = 2usize;
            if let Some(pos) = args.iter().position(|a| a == "--depth") {
                if pos + 1 < args.len() {
                    depth = args[pos + 1].parse().unwrap_or(2);
                }
            }

            let graph = SymbolGraph::index_directory(path);
            let report = graph.calculate_blast_radius(symbol, None, depth);
            println!(
                "Blast Radius for symbol '{}' [Risk: {}]:",
                symbol, report.risk_score
            );
            println!(
                "  Direct dependents ({}): {:?}",
                report.direct_dependents.len(),
                report.direct_dependents
            );
            println!(
                "  Affected files ({}): {:?}",
                report.affected_files.len(),
                report.affected_files
            );
        }

        "refs" => {
            if args.len() < 3 {
                eprintln!("Usage: locus refs <symbol_name> [<directory_path>]");
                process::exit(1);
            }
            let symbol = &args[2];
            let path = if args.len() > 3 { &args[3] } else { "." };
            let graph = SymbolGraph::index_directory(path);
            let refs = graph.find_references(symbol);
            println!("Found {} references to '{}':", refs.len(), symbol);
            for r in refs {
                println!("  - {}:{}: {}", r.file, r.line, r.context_snippet);
            }
        }

        "search" => {
            if args.len() < 3 {
                eprintln!("Usage: locus search <query> [<directory_path>]");
                process::exit(1);
            }
            let query = &args[2];
            let path = if args.len() > 3 { &args[3] } else { "." };
            let graph = SymbolGraph::index_directory(path);
            let matcher = HybridMatcher::new();
            matcher.index_graph(&graph);
            let res = matcher.search(query, 5);

            println!("Search results for '{}' ({:.2}ms):", query, res.latency_ms);
            for (idx, hit) in res.hits.iter().enumerate() {
                println!(
                    "  {}. {} (score: {:.2}) - {}",
                    idx + 1,
                    hit.symbol_name,
                    hit.score,
                    hit.file_path
                );
                println!("     {}", hit.signature);
            }
        }

        "taint" => {
            if args.len() < 3 {
                eprintln!("Usage: locus taint <file_path> [<symbol_name>]");
                process::exit(1);
            }
            let file_path = &args[2];
            let symbol = if args.len() > 3 { &args[3] } else { "*" };
            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading '{}': {}", file_path, e);
                    process::exit(1);
                }
            };
            let flow_reports = DataFlowTracker::analyze_source(file_path, symbol, &content);
            let null_reports = NullPropagationTracker::scan_nullable_flows(file_path, &content);

            println!("Taint analysis for '{}':", file_path);
            let total = flow_reports.len() + null_reports.len();
            if total == 0 {
                println!("  No unvalidated taint flows or unhandled null dereferences detected.");
            } else {
                for r in flow_reports {
                    println!(
                        "  [TAINT] {} -> {} sinks (Risk: {})",
                        r.source.variable,
                        r.sinks.len(),
                        r.violation_risk
                    );
                }
                for r in null_reports {
                    println!(
                        "  [NULL] Unhandled return from '{}' (Risk: {})",
                        r.source.symbol, r.violation_risk
                    );
                }
            }
        }

        "lease" => {
            if args.len() < 3 {
                eprintln!("Usage: locus lease <acquire|release|list> [arguments...]");
                process::exit(1);
            }
            let reg = LeaseRegistry::new();
            match args[2].as_str() {
                "acquire" if args.len() >= 5 => {
                    let fqn = &args[3];
                    let agent = &args[4];
                    let ttl = if args.len() > 5 {
                        args[5].parse().unwrap_or(60000)
                    } else {
                        60000
                    };
                    let status = reg.acquire(fqn, agent, ttl);
                    println!("{:?}", status);
                }
                "release" if args.len() >= 5 => {
                    let lease_id = &args[3];
                    let agent = &args[4];
                    let status = reg.release(lease_id, agent);
                    println!("{:?}", status);
                }
                "list" => {
                    let active = reg.list_active_leases();
                    println!("Active leases: {}", active.len());
                    for l in active {
                        println!(
                            "  - {} (held by '{}', expires in {}ms)",
                            l.fqn, l.holder_agent_id, l.ttl_ms
                        );
                    }
                }
                _ => {
                    eprintln!("Invalid lease command format.");
                    process::exit(1);
                }
            }
        }

        "patch" => {
            if args.len() < 3 {
                eprintln!(
                    "Usage: locus patch <file_path> --symbol <symbol_name> --with <new_code>"
                );
                process::exit(1);
            }
            let file_path = &args[2];
            let mut symbol_name: Option<String> = None;
            let mut new_code: Option<String> = None;

            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--symbol" if i + 1 < args.len() => {
                        symbol_name = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--with" if i + 1 < args.len() => {
                        new_code = Some(args[i + 1].clone());
                        i += 2;
                    }
                    _ => i += 1,
                }
            }

            let symbol = symbol_name.unwrap_or_default();
            let replacement = new_code.unwrap_or_default();

            if symbol.is_empty() || replacement.is_empty() {
                eprintln!("Error: Both --symbol and --with are required.");
                process::exit(1);
            }

            let content = match fs::read_to_string(file_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Error reading '{}': {}", file_path, e);
                    process::exit(1);
                }
            };

            let ext = Path::new(file_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let lang = Language::from_extension(ext);

            match AstDiffEngine::patch(&content, &symbol, &replacement, lang) {
                Ok(patched) => {
                    if let Err(e) = fs::write(file_path, &patched) {
                        eprintln!("Error writing '{}': {}", file_path, e);
                        process::exit(1);
                    }
                    println!(
                        "Successfully patched symbol '{}' in '{}'",
                        symbol, file_path
                    );
                }
                Err(e) => {
                    eprintln!("Patch failed: {}", e);
                    process::exit(1);
                }
            }
        }

        _ => {
            eprintln!("Unknown command: '{}'", args[1]);
            print_usage();
            process::exit(1);
        }
    }
}
