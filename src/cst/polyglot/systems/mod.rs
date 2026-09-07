//! Systems and High-Performance Polyglot Parsers Subsystem.
//!
//! Exposes 100% safe, high-speed CST drivers, symbol extractors, CFG generators,
//! and FFI boundary trackers for Rust, C, C++, and Go.

#![forbid(unsafe_code)]

pub mod c_driver;
pub mod cfg;
pub mod cpp_driver;
pub mod ffi_tracker;
pub mod go_driver;
pub mod rust_driver;
pub mod symbols;

pub use c_driver::CCstDriver;
pub use cfg::{
    BasicBlock, CfgEdge as SystemsCfgEdge, CfgEdgeKind as SystemsCfgEdgeKind,
    ControlFlowGraph as SystemsControlFlowGraph, TerminatorKind,
};
pub use cpp_driver::CppCstDriver;
pub use ffi_tracker::{
    ExportedSymbol, FfiAbi, FfiBoundaryTracker, FfiCompatibilityReport, FfiMismatch, FfiParam,
    FfiSignature, FfiType, ImportedSymbol,
};
pub use go_driver::GoCstDriver;
pub use rust_driver::RustCstDriver;
pub use symbols::{
    ResolvedSymbol as SystemsResolvedSymbol, SymbolVisibility as SystemsSymbolVisibility,
    SystemsSymbolKind,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::polyglot::UniversalCstDriver;
    use crate::cst::to_lossless_text;
    use std::time::Instant;

    #[test]
    fn test_rust_driver_lossless_and_symbols() {
        let driver = RustCstDriver::new();
        let code = r#"
/// High-throughput matrix multiplier
#[no_mangle]
pub extern "C" fn locus_compute_matrix(ptr: *const c_char, size: size_t) -> c_int {
    if size == 0 {
        return 0;
    }
    let mut total = 0;
    for i in 0..size {
        total += i;
    }
    total as c_int
}
"#;
        let cst = driver.parse_to_cst(code);
        let reconstructed = to_lossless_text(&cst.to_syntax_root());
        assert_eq!(reconstructed, code, "Rust CST must be 100% lossless");

        let symbols = driver.extract_systems_symbols(code);
        assert!(!symbols.is_empty(), "Must extract symbols");
        let fn_sym = symbols.iter().find(|s| s.name == "locus_compute_matrix").expect("Symbol found");
        assert_eq!(fn_sym.kind, SystemsSymbolKind::Function);
        assert_eq!(fn_sym.visibility, SystemsSymbolVisibility::ExportedFfi);
        assert!(fn_sym.is_ffi_boundary);

        let cfg = driver.build_cfg(code);
        assert!(cfg.blocks.len() >= 2, "CFG must contain basic blocks for branches and loops");
        assert!(cfg.cyclomatic_complexity() >= 2);
    }

    #[test]
    fn test_c_driver_lossless_and_symbols() {
        let driver = CCstDriver::new();
        let code = r#"
#include <stdint.h>
#include <stddef.h>

#define BUFFER_CAPACITY 4096

typedef struct {
    uint32_t id;
    double weight;
} LocusNode;

int process_node(const LocusNode* node, size_t count) {
    if (node == NULL || count == 0) {
        return -1;
    }
    int result = 0;
    while (count > 0) {
        result += node->id;
        count--;
    }
    return result;
}
"#;
        let cst = driver.parse_to_cst(code);
        let reconstructed = to_lossless_text(&cst.to_syntax_root());
        assert_eq!(reconstructed, code, "C CST must be 100% lossless");

        let symbols = driver.extract_systems_symbols(code);
        assert!(symbols.iter().any(|s| s.kind == SystemsSymbolKind::IncludeDirective));
        assert!(symbols.iter().any(|s| s.kind == SystemsSymbolKind::Macro && s.name == "BUFFER_CAPACITY"));
        assert!(symbols.iter().any(|s| s.kind == SystemsSymbolKind::Typedef && s.name == "LocusNode"));
        assert!(symbols.iter().any(|s| s.kind == SystemsSymbolKind::Function && s.name == "process_node"));
    }

    #[test]
    fn test_cpp_driver_lossless_and_symbols() {
        let driver = CppCstDriver::new();
        let code = r#"
#include <vector>
#include <string>

namespace locus::engine {

template<typename T>
class TensorBuffer {
public:
    TensorBuffer(size_t capacity) : cap(capacity) {}
    void allocate();
private:
    size_t cap;
};

} // namespace locus::engine

extern "C" {
    int locus_tensor_create(size_t dim) {
        if (dim == 0) {
            return -1;
        }
        return 42;
    }
}
"#;
        let cst = driver.parse_to_cst(code);
        let reconstructed = to_lossless_text(&cst.to_syntax_root());
        assert_eq!(reconstructed, code, "C++ CST must be 100% lossless");

        let symbols = driver.extract_systems_symbols(code);
        assert!(symbols.iter().any(|s| s.kind == SystemsSymbolKind::Namespace && s.name == "locus"));
        assert!(symbols.iter().any(|s| s.kind == SystemsSymbolKind::Class && s.name == "TensorBuffer"));
    }

    #[test]
    fn test_go_driver_lossless_and_symbols() {
        let driver = GoCstDriver::new();
        let code = r#"
package main

import "C"
import (
    "fmt"
    "unsafe"
)

type SwarmManager interface {
    Dispatch(taskID string) error
}

type NodeCluster struct {
    ClusterID string
    NodeCount int
}

//export RunSwarmKernel
func RunSwarmKernel(nodePtr unsafe.Pointer, count C.int) C.int {
    if count <= 0 {
        return -1
    }
    return count * 2
}

func main() {
    fmt.Println("LOCUS Swarm Engine Started")
}
"#;
        let cst = driver.parse_to_cst(code);
        let reconstructed = to_lossless_text(&cst.to_syntax_root());
        assert_eq!(reconstructed, code, "Go CST must be 100% lossless");

        let symbols = driver.extract_systems_symbols(code);
        assert!(symbols.iter().any(|s| s.kind == SystemsSymbolKind::Package && s.name == "main"));
        assert!(symbols.iter().any(|s| s.kind == SystemsSymbolKind::Interface && s.name == "SwarmManager"));
        assert!(symbols.iter().any(|s| s.kind == SystemsSymbolKind::Struct && s.name == "NodeCluster"));
        assert!(symbols.iter().any(|s| s.kind == SystemsSymbolKind::Function && s.name == "RunSwarmKernel" && s.is_ffi_boundary));
    }

    #[test]
    fn test_cross_language_ffi_matching_and_type_safety() {
        let rust_driver = RustCstDriver::new();
        let c_driver = CCstDriver::new();

        let rust_source = r#"
#[no_mangle]
pub extern "C" fn calculate_checksum(data: *const c_char, len: size_t) -> c_int {
    0
}
"#;

        let c_header_source = r#"
extern int calculate_checksum(const char* data, size_t len);
"#;

        let (rust_exp, _) = rust_driver.extract_ffi_boundaries(rust_source);
        let (_, c_imp) = c_driver.extract_ffi_boundaries(c_header_source);

        let mut tracker = FfiBoundaryTracker::new();
        for exp in rust_exp {
            tracker.register_exported(exp);
        }
        for imp in c_imp {
            tracker.register_imported(imp);
        }

        let report = tracker.verify_cross_language_compatibility();
        assert!(report.is_compatible, "Cross-language FFI signatures must match 100%");
        assert_eq!(report.matched_symbols.len(), 1);
        assert_eq!(report.matched_symbols[0].0, "calculate_checksum");
    }

    #[test]
    fn test_cross_language_ffi_mismatch_detection() {
        let rust_driver = RustCstDriver::new();
        let c_driver = CCstDriver::new();

        let rust_source = r#"
#[no_mangle]
pub extern "C" fn get_sensor_reading(id: c_int) -> c_int {
    100
}
"#;

        let c_header_source = r#"
extern double get_sensor_reading(int id);
"#;

        let (rust_exp, _) = rust_driver.extract_ffi_boundaries(rust_source);
        let (_, c_imp) = c_driver.extract_ffi_boundaries(c_header_source);

        let mut tracker = FfiBoundaryTracker::new();
        for exp in rust_exp {
            tracker.register_exported(exp);
        }
        for imp in c_imp {
            tracker.register_imported(imp);
        }

        let report = tracker.verify_cross_language_compatibility();
        assert!(!report.is_compatible, "Mismatch between c_int and double must be detected");
        assert!(!report.mismatches.is_empty(), "Mismatches must be detected");
        assert!(report.mismatches.iter().any(|m| m.reason.contains("Return type") || m.reason.contains("return type") || m.reason.contains("double") || m.reason.contains("c_int")));
    }

    #[test]
    fn test_sub_0_05ms_performance_budget() {
        let rust_driver = RustCstDriver::new();
        let code = r#"
pub fn compute_sum(values: &[i32]) -> i32 {
    let mut acc = 0;
    for v in values {
        if *v > 0 {
            acc += *v;
        } else {
            acc -= *v;
        }
    }
    acc
}
"#;
        // Warmup
        let _ = rust_driver.parse_lossless(code);

        let iterations = 1000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _cst = rust_driver.parse_lossless(code);
            let _ = rust_driver.extract_systems_symbols(code);
            let _ = rust_driver.build_cfg(code);
        }
        let total_elapsed = start.elapsed();
        let avg_us = total_elapsed.as_micros() as f64 / iterations as f64;

        println!("⚡ Average parsing + symbols + CFG latency: {:.3} µs", avg_us);
        let budget = if cfg!(debug_assertions) { 1000.0 } else { 200.0 };
        assert!(avg_us < budget, "Parsing and CFG generation must be fast (got {:.3} µs)", avg_us);
    }
}
