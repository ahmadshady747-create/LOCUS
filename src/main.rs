//! locus CLI — High-speed deterministic verification and AST semantic tooling.

use std::env;
use std::fs;
use std::path::Path;
use std::process;
use std::time::Instant;

use locus_engine::{
    run_stdio_server, AstDiffEngine, AstGuard, ContextSlicer, ContractSynthesizer, Language, SymbolGraph,
};

fn print_usage() {
    eprintln!(
        r#"locus-engine CLI v1.0.0

USAGE:
    locus check <file_path>
    locus skeleton <file_path>
    locus contract <intent> [--lang <lang>] [--target <file_path>]
    locus slice <symbol_name> <file_path> [--depth <depth>]
    locus graph <directory_path>
    locus impact <symbol_name> <file_or_dir> [--depth <depth>]
    locus refs <symbol_name> <directory_path>
    locus patch <file_path> --symbol <symbol_name> --with <new_code>
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
        "mcp" => {
            if let Err(e) = run_stdio_server() {
                eprintln!("MCP Server error: {}", e);
                process::exit(1);
            }
        }

        "contract" => {
            if args.len() < 3 {
                eprintln!("Usage: locus contract <intent> [--lang <lang>] [--target <target_path>]");
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
            let ext = Path::new(file_path).extension().and_then(|e| e.to_str()).unwrap_or("");
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
            println!("\n+-------------------------------------------------------------+");
            println!("|                  LOCUS AST GUARD VERIFICATION               |");
            println!("+-------------------------------------------------------------+");
            println!(" Target File: {}", file_path);
            println!(" Verified Latency: {:.4} ms", report.latency_ms);
            if report.passed {
                println!(" Status: [PASS] All Deterministic Safety Invariants Validated");
            } else {
                println!(" Status: [FAIL] Invariant Violation Detected");
                if let Some(v) = report.violation {
                    println!(" Violation Kind: {}", v);
                }
                if let Some(detail) = report.detail {
                    println!(" Violation Detail: {}", detail);
                }
            }
            println!("+-------------------------------------------------------------+\n");

            if !report.passed {
                process::exit(2);
            }
        }

        "graph" => {
            if args.len() < 3 {
                eprintln!("Error: Missing <directory_path> for 'graph' command.");
                process::exit(1);
            }
            let dir_path = &args[2];
            let graph = SymbolGraph::index_directory(dir_path);
            let health = graph.analyze_architectural_health();

            println!("\n+-------------------------------------------------------------+");
            println!("|                   LOCUS SYMBOL GRAPH INDEX                  |");
            println!("+-------------------------------------------------------------+");
            println!(" Indexed Root: {}", dir_path);
            println!(" Total Indexed Files: {}", health.total_files);
            println!(" Extracted AST Symbols: {}", health.total_symbols);
            println!(" Cross-Symbol Dependency Edges: {}", health.total_edges);
            println!(" Circular Dependency Cycles: {}", health.circular_dependencies.len());
            if !health.circular_dependencies.is_empty() {
                for (idx, cycle) in health.circular_dependencies.iter().enumerate() {
                    println!("   Cycle #{}: {}", idx + 1, cycle.join(" -> "));
                }
            }
            println!(" Orphan and Unused Exports: {}", health.orphan_exports.len());
            if !health.orphan_exports.is_empty() {
                for orphan in health.orphan_exports.iter().take(5) {
                    println!("   - {}", orphan);
                }
                if health.orphan_exports.len() > 5 {
                    println!("   ... and {} more", health.orphan_exports.len() - 5);
                }
            }
            println!(" Indexing Latency: {:.2} ms", health.latency_ms);
            println!("+-------------------------------------------------------------+\n");
        }

        "impact" => {
            if args.len() < 4 {
                eprintln!("Usage: locus impact <symbol_name> <file_or_directory> [--depth <depth>]");
                process::exit(1);
            }
            let symbol = &args[2];
            let target = &args[3];
            let mut depth = 2usize;
            if args.len() > 5 && args[4] == "--depth" {
                depth = args[5].parse().unwrap_or(2);
            }

            let path_obj = Path::new(target);
            let (graph, file_opt) = if path_obj.is_dir() {
                (SymbolGraph::index_directory(target), None)
            } else {
                let dir = path_obj.parent().unwrap_or(Path::new("."));
                let g = SymbolGraph::index_directory(dir);
                let rel = path_obj.to_string_lossy().replace('\\', "/");
                (g, Some(rel))
            };

            let report = graph.calculate_blast_radius(symbol, file_opt.as_deref(), depth);

            println!("\n+-------------------------------------------------------------+");
            println!("|                  LOCUS BLAST RADIUS REPORT                  |");
            println!("+-------------------------------------------------------------+");
            println!(" Target Symbol: {}", report.symbol);
            println!(" Origin File: {}", report.origin_file);
            println!(" Breaking Change Risk: [{}]", report.risk_score);
            println!(" Inbound Reference Sites: {}", report.reference_count);
            println!(" Direct Dependents ({}):", report.direct_dependents.len());
            for dep in &report.direct_dependents {
                println!("   - {}", dep);
            }
            println!(" Transitive Dependents ({}):", report.transitive_dependents.len());
            for dep in &report.transitive_dependents {
                println!("   - {}", dep);
            }
            println!(" Impacted Files Set ({}):", report.affected_files.len());
            for f in &report.affected_files {
                println!("   * {}", f);
            }
            println!(" Analysis Latency: {:.4} ms", report.latency_ms);
            println!("+-------------------------------------------------------------+\n");
        }

        "refs" => {
            if args.len() < 4 {
                eprintln!("Usage: locus refs <symbol_name> <directory_path>");
                process::exit(1);
            }
            let symbol = &args[2];
            let dir_path = &args[3];

            let start = Instant::now();
            let graph = SymbolGraph::index_directory(dir_path);
            let refs = graph.find_references(symbol);
            let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

            println!("\n+-------------------------------------------------------------+");
            println!("|                   LOCUS SYMBOL REFERENCES                   |");
            println!("+-------------------------------------------------------------+");
            println!(" Target Symbol: {}", symbol);
            println!(" Total Occurrences Found: {}", refs.len());
            println!(" Query Latency: {:.4} ms", elapsed_ms);
            for (idx, r) in refs.iter().enumerate() {
                println!(" {:2}. {}:{} | {}", idx + 1, r.file, r.line, r.context_snippet);
            }
            println!("+-------------------------------------------------------------+\n");
        }

        "patch" => {
            if args.len() < 7 {
                eprintln!("Usage: locus patch <file_path> --symbol <symbol_name> --with <new_code>");
                process::exit(1);
            }
            let file_path = &args[2];
            let mut symbol_name: Option<String> = None;
            let mut new_code: Option<String> = None;

            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--symbol" => {
                        if i + 1 < args.len() {
                            symbol_name = Some(args[i + 1].clone());
                            i += 2;
                        } else {
                            i += 1;
                        }
                    }
                    "--with" => {
                        if i + 1 < args.len() {
                            new_code = Some(args[i + 1].clone());
                            i += 2;
                        } else {
                            i += 1;
                        }
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
                    println!("Successfully patched symbol '{}' in '{}'", symbol, file_path);
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
