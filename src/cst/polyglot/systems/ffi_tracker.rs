//! Cross-Language FFI Boundary Tracker and ABI Type Verifier.
//!
//! Provides deterministic verification of exported (`#[no_mangle]`, `extern "C"`, `//export`)
//! and imported (`extern "C" { ... }`, `import "C"`, `extern`) symbols across Rust, C, C++, and Go.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::cst::green::TextRange;

/// Application Binary Interface (ABI) calling convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FfiAbi {
    C,
    Cdecl,
    Stdcall,
    Fastcall,
    Rust,
    Go,
    System,
}

impl fmt::Display for FfiAbi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            FfiAbi::C => "C",
            FfiAbi::Cdecl => "cdecl",
            FfiAbi::Stdcall => "stdcall",
            FfiAbi::Fastcall => "fastcall",
            FfiAbi::Rust => "Rust",
            FfiAbi::Go => "Go",
            FfiAbi::System => "system",
        };
        write!(f, "{}", s)
    }
}

/// Canonical FFI type representation for cross-language compatibility verification.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FfiType {
    Void,
    Bool,
    Int { bits: usize, signed: bool },
    Float { bits: usize },
    Pointer { is_const: bool, pointee: Box<FfiType> },
    RawPointer,
    Struct(String),
    Enum(String),
    Array { element: Box<FfiType>, len: Option<usize> },
    Custom(String),
}

impl FfiType {
    /// Normalizes a type string from Rust syntax into canonical `FfiType`.
    pub fn from_rust(raw: &str) -> Self {
        let t = raw.trim();
        if t == "()" || t == "c_void" || t.is_empty() {
            return FfiType::Void;
        }
        if t == "bool" {
            return FfiType::Bool;
        }
        if t == "i8" || t == "c_char" || t == "c_schar" {
            return FfiType::Int { bits: 8, signed: true };
        }
        if t == "u8" || t == "c_uchar" {
            return FfiType::Int { bits: 8, signed: false };
        }
        if t == "i16" || t == "c_short" {
            return FfiType::Int { bits: 16, signed: true };
        }
        if t == "u16" || t == "c_ushort" {
            return FfiType::Int { bits: 16, signed: false };
        }
        if t == "i32" || t == "c_int" {
            return FfiType::Int { bits: 32, signed: true };
        }
        if t == "u32" || t == "c_uint" {
            return FfiType::Int { bits: 32, signed: false };
        }
        if t == "i64" || t == "c_longlong" || t == "int64_t" {
            return FfiType::Int { bits: 64, signed: true };
        }
        if t == "u64" || t == "c_ulonglong" || t == "uint64_t" {
            return FfiType::Int { bits: 64, signed: false };
        }
        if t == "isize" || t == "ssize_t" {
            return FfiType::Int { bits: 64, signed: true };
        }
        if t == "usize" || t == "size_t" {
            return FfiType::Int { bits: 64, signed: false };
        }
        if t == "f32" || t == "c_float" {
            return FfiType::Float { bits: 32 };
        }
        if t == "f64" || t == "c_double" {
            return FfiType::Float { bits: 64 };
        }
        if let Some(inner) = t.strip_prefix("*const ") {
            return FfiType::Pointer {
                is_const: true,
                pointee: Box::new(Self::from_rust(inner)),
            };
        }
        if let Some(inner) = t.strip_prefix("*mut ") {
            return FfiType::Pointer {
                is_const: false,
                pointee: Box::new(Self::from_rust(inner)),
            };
        }
        if t.starts_with('*') {
            return FfiType::RawPointer;
        }

        FfiType::Custom(t.to_string())
    }

    /// Normalizes a type string from C/C++ syntax into canonical `FfiType`.
    pub fn from_c(raw: &str) -> Self {
        let t = raw.trim();
        if t == "void" || t.is_empty() {
            return FfiType::Void;
        }
        if t == "bool" || t == "_Bool" {
            return FfiType::Bool;
        }
        if t == "char" || t == "signed char" || t == "int8_t" {
            return FfiType::Int { bits: 8, signed: true };
        }
        if t == "unsigned char" || t == "uint8_t" || t == "uint8" {
            return FfiType::Int { bits: 8, signed: false };
        }
        if t == "short" || t == "short int" || t == "int16_t" {
            return FfiType::Int { bits: 16, signed: true };
        }
        if t == "unsigned short" || t == "uint16_t" {
            return FfiType::Int { bits: 16, signed: false };
        }
        if t == "int" || t == "signed int" || t == "int32_t" || t == "long" {
            return FfiType::Int { bits: 32, signed: true };
        }
        if t == "unsigned int" || t == "uint32_t" || t == "unsigned" || t == "unsigned long" {
            return FfiType::Int { bits: 32, signed: false };
        }
        if t == "long long" || t == "int64_t" || t == "ssize_t" {
            return FfiType::Int { bits: 64, signed: true };
        }
        if t == "unsigned long long" || t == "uint64_t" || t == "size_t" {
            return FfiType::Int { bits: 64, signed: false };
        }
        if t == "float" {
            return FfiType::Float { bits: 32 };
        }
        if t == "double" {
            return FfiType::Float { bits: 64 };
        }
        if t.starts_with("const ") && t.ends_with('*') {
            if let Some(without_const) = t.strip_prefix("const ") {
                if let Some(inner) = without_const.strip_suffix('*') {
                    return FfiType::Pointer {
                        is_const: true,
                        pointee: Box::new(Self::from_c(inner.trim())),
                    };
                }
            }
        }
        if let Some(inner) = t.strip_suffix('*') {
            let inner_trimmed = inner.trim();
            let (is_const, pointee_str) = if let Some(after_const) = inner_trimmed.strip_prefix("const ") {
                (true, after_const.trim())
            } else {
                (false, inner_trimmed)
            };
            return FfiType::Pointer {
                is_const,
                pointee: Box::new(Self::from_c(pointee_str)),
            };
        }

        FfiType::Custom(t.to_string())
    }

    /// Normalizes a type string from Go syntax into canonical `FfiType`.
    pub fn from_go(raw: &str) -> Self {
        let t = raw.trim();
        if t.is_empty() {
            return FfiType::Void;
        }
        if t == "bool" {
            return FfiType::Bool;
        }
        if t == "C.char" || t == "C.schar" || t == "int8" || t == "byte" {
            return FfiType::Int { bits: 8, signed: true };
        }
        if t == "C.uchar" || t == "uint8" {
            return FfiType::Int { bits: 8, signed: false };
        }
        if t == "C.short" || t == "int16" {
            return FfiType::Int { bits: 16, signed: true };
        }
        if t == "C.ushort" || t == "uint16" {
            return FfiType::Int { bits: 16, signed: false };
        }
        if t == "C.int" || t == "int32" || t == "int" {
            return FfiType::Int { bits: 32, signed: true };
        }
        if t == "C.uint" || t == "uint32" || t == "uint" {
            return FfiType::Int { bits: 32, signed: false };
        }
        if t == "C.longlong" || t == "int64" {
            return FfiType::Int { bits: 64, signed: true };
        }
        if t == "C.ulonglong" || t == "uint64" || t == "C.size_t" || t == "uintptr" {
            return FfiType::Int { bits: 64, signed: false };
        }
        if t == "C.float" || t == "float32" {
            return FfiType::Float { bits: 32 };
        }
        if t == "C.double" || t == "float64" {
            return FfiType::Float { bits: 64 };
        }
        if t == "unsafe.Pointer" {
            return FfiType::Pointer {
                is_const: false,
                pointee: Box::new(FfiType::Void),
            };
        }
        if let Some(inner) = t.strip_prefix('*') {
            return FfiType::Pointer {
                is_const: false,
                pointee: Box::new(Self::from_go(inner)),
            };
        }

        FfiType::Custom(t.to_string())
    }

    /// Verifies binary layout compatibility between two types.
    pub fn is_layout_compatible(&self, other: &FfiType) -> bool {
        if self == other {
            return true;
        }

        match (self, other) {
            (FfiType::Void, FfiType::Void) => true,
            (FfiType::Bool, FfiType::Bool) => true,
            (
                FfiType::Int { bits: b1, signed: s1 },
                FfiType::Int { bits: b2, signed: s2 },
            ) => b1 == b2 && s1 == s2,
            (
                FfiType::Float { bits: b1 },
                FfiType::Float { bits: b2 },
            ) => b1 == b2,
            (
                FfiType::Pointer { pointee: p1, .. },
                FfiType::Pointer { pointee: p2, .. },
            ) => {
                if **p1 == FfiType::Void || **p2 == FfiType::Void {
                    true
                } else {
                    p1.is_layout_compatible(p2)
                }
            }
            (FfiType::RawPointer, FfiType::Pointer { .. })
            | (FfiType::Pointer { .. }, FfiType::RawPointer)
            | (FfiType::RawPointer, FfiType::RawPointer) => true,
            (FfiType::Custom(c1), FfiType::Custom(c2)) => c1 == c2,
            _ => false,
        }
    }
}

/// An individual parameter in an FFI function signature.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FfiParam {
    pub name: String,
    pub ffi_type: FfiType,
}

/// Complete FFI function signature.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FfiSignature {
    pub abi: FfiAbi,
    pub params: Vec<FfiParam>,
    pub return_type: FfiType,
    pub is_variadic: bool,
}

/// Symbol exported to other languages via FFI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportedSymbol {
    pub name: String,
    pub link_name: String,
    pub language: String,
    pub signature: FfiSignature,
    pub file: String,
    pub range: TextRange,
}

/// Symbol imported from another language via FFI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedSymbol {
    pub name: String,
    pub link_name: String,
    pub language: String,
    pub signature: FfiSignature,
    pub target_library: Option<String>,
    pub file: String,
    pub range: TextRange,
}

/// Discrepancy report between exported and imported symbol declarations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FfiMismatch {
    pub symbol_name: String,
    pub exporter_lang: String,
    pub importer_lang: String,
    pub reason: String,
    pub expected: String,
    pub actual: String,
}

/// Comprehensive cross-language FFI verification report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FfiCompatibilityReport {
    pub matched_symbols: Vec<(String, String, String)>,
    pub mismatches: Vec<FfiMismatch>,
    pub is_compatible: bool,
}

/// Cross-Language FFI Boundary Registry & Invariant Validator.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FfiBoundaryTracker {
    pub exported: HashMap<String, ExportedSymbol>,
    pub imported: HashMap<String, Vec<ImportedSymbol>>,
}

impl FfiBoundaryTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a newly analyzed exported FFI symbol.
    pub fn register_exported(&mut self, symbol: ExportedSymbol) {
        self.exported.insert(symbol.link_name.clone(), symbol);
    }

    /// Registers a newly analyzed imported FFI declaration.
    pub fn register_imported(&mut self, symbol: ImportedSymbol) {
        self.imported
            .entry(symbol.link_name.clone())
            .or_default()
            .push(symbol);
    }

    /// Verifies a single pair of exported and imported symbols for strict binary compatibility.
    pub fn verify_pair(
        exported: &ExportedSymbol,
        imported: &ImportedSymbol,
    ) -> Result<(), FfiMismatch> {
        if !exported.signature.return_type.is_layout_compatible(&imported.signature.return_type) {
            return Err(FfiMismatch {
                symbol_name: exported.name.clone(),
                exporter_lang: exported.language.clone(),
                importer_lang: imported.language.clone(),
                reason: "Return type binary layout mismatch".to_string(),
                expected: format!("{:?}", exported.signature.return_type),
                actual: format!("{:?}", imported.signature.return_type),
            });
        }

        if exported.signature.params.len() != imported.signature.params.len() {
            return Err(FfiMismatch {
                symbol_name: exported.name.clone(),
                exporter_lang: exported.language.clone(),
                importer_lang: imported.language.clone(),
                reason: format!(
                    "Parameter arity mismatch: exported {} params, imported {} params",
                    exported.signature.params.len(),
                    imported.signature.params.len()
                ),
                expected: format!("{} parameters", exported.signature.params.len()),
                actual: format!("{} parameters", imported.signature.params.len()),
            });
        }

        for (i, (exp_param, imp_param)) in exported
            .signature
            .params
            .iter()
            .zip(imported.signature.params.iter())
            .enumerate()
        {
            if !exp_param.ffi_type.is_layout_compatible(&imp_param.ffi_type) {
                return Err(FfiMismatch {
                    symbol_name: exported.name.clone(),
                    exporter_lang: exported.language.clone(),
                    importer_lang: imported.language.clone(),
                    reason: format!("Parameter #{} ({}) type layout mismatch", i, exp_param.name),
                    expected: format!("{:?}", exp_param.ffi_type),
                    actual: format!("{:?}", imp_param.ffi_type),
                });
            }
        }

        Ok(())
    }

    /// Performs full cross-language FFI boundary validation across all registered symbols.
    pub fn verify_cross_language_compatibility(&self) -> FfiCompatibilityReport {
        let mut matched_symbols = Vec::new();
        let mut mismatches = Vec::new();

        for (link_name, import_list) in &self.imported {
            if let Some(exported_symbol) = self.exported.get(link_name) {
                for imported_symbol in import_list {
                    match Self::verify_pair(exported_symbol, imported_symbol) {
                        Ok(()) => {
                            matched_symbols.push((
                                link_name.clone(),
                                exported_symbol.language.clone(),
                                imported_symbol.language.clone(),
                            ));
                        }
                        Err(mismatch) => {
                            mismatches.push(mismatch);
                        }
                    }
                }
            }
        }

        let is_compatible = mismatches.is_empty();

        FfiCompatibilityReport {
            matched_symbols,
            mismatches,
            is_compatible,
        }
    }
}
