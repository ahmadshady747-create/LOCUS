//! Domain models, security policies, and cryptographic certificates for Dynamic Meta-MCP Tool Verification.
//!
//! Enforces zero-unsafe static capability checks, taint tracking, and cryptographic
//! tool safety certification in 100% pure safe Rust.

#![forbid(unsafe_code)]

use crate::meta_mcp::sandbox::types::ToolCapability;
use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Security Policies & Violations
// ---------------------------------------------------------------------------

/// Declarative security policy rules enforced on dynamic tool source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityPolicyRule {
    /// Forbids invoking `std::process::Command`, `libc`, `fork`, `exec`, or process spawning.
    DenySystemProcessSpawn,
    /// Forbids arbitrary disk write operations unless `WriteWorkspace` capability is granted.
    DenyArbitraryDiskWrite,
    /// Forbids unbounded TCP/UDP/HTTP networking unless `NetworkRestricted` capability is granted.
    DenyUnboundedNetworkAccess,
    /// Requires non-panicking input boundary guards on all parsed JSON arguments.
    RequireStrictInputSanitization,
    /// Enforces non-infinite loop constructs and memory ceilings.
    EnforceMemoryCeiling,
}

/// Categorized security violation kinds detected during static AST analysis or sandbox fuzzing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SecurityViolationKind {
    /// Attempt to access or mutate unauthorized files on the filesystem.
    UnauthorizedFileAccess,
    /// Attempt to spawn external processes or execute shell commands.
    ProcessExecutionAttempt,
    /// Detected infinite or unbounded loop construct.
    UnboundedLoopDetected,
    /// Operation attempted without corresponding declared capability token.
    CapabilityMismatch,
    /// Tool execution exceeded designated timeout threshold.
    ExecutionTimeoutViolation,
}

impl fmt::Display for SecurityViolationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnauthorizedFileAccess => write!(f, "UNAUTHORIZED_FILE_ACCESS"),
            Self::ProcessExecutionAttempt => write!(f, "PROCESS_EXECUTION_ATTEMPT"),
            Self::UnboundedLoopDetected => write!(f, "UNBOUNDED_LOOP_DETECTED"),
            Self::CapabilityMismatch => write!(f, "CAPABILITY_MISMATCH"),
            Self::ExecutionTimeoutViolation => write!(f, "EXECUTION_TIMEOUT_VIOLATION"),
        }
    }
}

/// Detailed diagnostic report for a detected security violation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityViolation {
    pub violation_kind: SecurityViolationKind,
    pub target_node: String,
    pub line_offset: usize,
    pub description: String,
    pub remediation_hint: String,
}

impl SecurityViolation {
    pub fn new(
        violation_kind: SecurityViolationKind,
        target_node: impl Into<String>,
        line_offset: usize,
        description: impl Into<String>,
        remediation_hint: impl Into<String>,
    ) -> Self {
        Self {
            violation_kind,
            target_node: target_node.into(),
            line_offset,
            description: description.into(),
            remediation_hint: remediation_hint.into(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool Security Certificate
// ---------------------------------------------------------------------------

/// Cryptographic certification proving a dynamic Meta-MCP tool passed 100% AST safety & capability audits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolSecurityCertificate {
    pub tool_id: u64,
    pub code_sha256: String,
    pub granted_capabilities: Vec<ToolCapability>,
    pub verified_at_timestamp: u64,
    pub issuer_signature: String,
}

impl ToolSecurityCertificate {
    pub fn new(
        tool_id: u64,
        source_code: &str,
        granted_capabilities: Vec<ToolCapability>,
        timestamp: u64,
    ) -> Self {
        let sha256 = compute_sha256(source_code.as_bytes());
        let sig_seed = format!("{}:{}:{:?}:{}", tool_id, sha256, granted_capabilities, timestamp);
        let issuer_signature = format!("LOCUS-VERIFIED:{}", compute_sha256(sig_seed.as_bytes()));

        Self {
            tool_id,
            code_sha256: sha256,
            granted_capabilities,
            verified_at_timestamp: timestamp,
            issuer_signature,
        }
    }
}

/// Complete outcome of the Meta-MCP Tool Security Verification pipeline.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolVerificationReport {
    pub passed: bool,
    pub violations: Vec<SecurityViolation>,
    pub certificate: Option<ToolSecurityCertificate>,
    pub latency_ms: f64,
}

impl ToolVerificationReport {
    pub fn passed(certificate: ToolSecurityCertificate, latency_ms: f64) -> Self {
        Self {
            passed: true,
            violations: Vec::new(),
            certificate: Some(certificate),
            latency_ms,
        }
    }

    pub fn failed(violations: Vec<SecurityViolation>, latency_ms: f64) -> Self {
        Self {
            passed: false,
            violations,
            certificate: None,
            latency_ms,
        }
    }
}

// ---------------------------------------------------------------------------
// Pure Safe Rust SHA-256 Implementation (FIPS 180-4)
// ---------------------------------------------------------------------------

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Computes the standard cryptographic SHA-256 hash in pure 100% safe Rust.
pub fn compute_sha256(data: &[u8]) -> String {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64) * 8;

    msg.push(0x80);
    while (msg.len() + 8) % 64 != 0 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, w_elem) in w.iter_mut().take(16).enumerate() {
            let start = i * 4;
            *w_elem = u32::from_be_bytes([chunk[start], chunk[start + 1], chunk[start + 2], chunk[start + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_var = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_var.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_var = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_var);
    }

    let mut res = String::with_capacity(64);
    for val in h {
        res.push_str(&format!("{:08x}", val));
    }
    res
}
