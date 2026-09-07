//! Domain models, AST assertions, tri-state verdicts, and cryptographic certificates
//! for the Dynamic Proof Verification Engine (Task 10.1).
//!
//! Enforces mathematical invariant guarantees, lossless serialization,
//! and cryptographic claim-proof bindings in 100% pure safe Rust.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// 1. Proof Assertions (AST & DataFlow Grammar)
// ---------------------------------------------------------------------------

/// Structural, semantic, and relational assertions evaluated by the Dynamic Proof Verifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProofAssertion {
    /// Asserts that a syntax or AST node of a given kind exists (optionally matching an identifier).
    NodeExists {
        node_kind: String,
        identifier: Option<String>,
    },
    /// Asserts that a specified property on a node matches an expected value.
    PropertyEquals {
        node_id: String,
        property: String,
        expected_value: String,
    },
    /// Asserts hierarchical AST nesting (parent contains child of specified kind).
    ContainsSubNode {
        parent_kind: String,
        child_kind: String,
    },
    /// Asserts data flow relationship between source and sink symbols with optional sanitizer requirement.
    DataFlowRelation {
        source_symbol: String,
        sink_symbol: String,
        must_pass_sanitizer: bool,
    },
    /// Asserts that two source byte/token ranges do not overlap/intersect.
    RangeIntersectionEmpty {
        range_a: (usize, usize),
        range_b: (usize, usize),
    },
}

impl ProofAssertion {
    /// Constructs a `NodeExists` assertion.
    pub fn node_exists(node_kind: impl Into<String>, identifier: Option<impl Into<String>>) -> Self {
        Self::NodeExists {
            node_kind: node_kind.into(),
            identifier: identifier.map(|id| id.into()),
        }
    }

    /// Constructs a `PropertyEquals` assertion.
    pub fn property_equals(
        node_id: impl Into<String>,
        property: impl Into<String>,
        expected_value: impl Into<String>,
    ) -> Self {
        Self::PropertyEquals {
            node_id: node_id.into(),
            property: property.into(),
            expected_value: expected_value.into(),
        }
    }

    /// Constructs a `ContainsSubNode` assertion.
    pub fn contains_sub_node(
        parent_kind: impl Into<String>,
        child_kind: impl Into<String>,
    ) -> Self {
        Self::ContainsSubNode {
            parent_kind: parent_kind.into(),
            child_kind: child_kind.into(),
        }
    }

    /// Constructs a `DataFlowRelation` assertion.
    pub fn data_flow_relation(
        source_symbol: impl Into<String>,
        sink_symbol: impl Into<String>,
        must_pass_sanitizer: bool,
    ) -> Self {
        Self::DataFlowRelation {
            source_symbol: source_symbol.into(),
            sink_symbol: sink_symbol.into(),
            must_pass_sanitizer,
        }
    }

    /// Constructs a `RangeIntersectionEmpty` assertion.
    pub fn range_intersection_empty(range_a: (usize, usize), range_b: (usize, usize)) -> Self {
        Self::RangeIntersectionEmpty { range_a, range_b }
    }

    /// Returns a human-readable identifier of the assertion type.
    pub fn assertion_type(&self) -> &'static str {
        match self {
            Self::NodeExists { .. } => "NodeExists",
            Self::PropertyEquals { .. } => "PropertyEquals",
            Self::ContainsSubNode { .. } => "ContainsSubNode",
            Self::DataFlowRelation { .. } => "DataFlowRelation",
            Self::RangeIntersectionEmpty { .. } => "RangeIntersectionEmpty",
        }
    }
}

impl fmt::Display for ProofAssertion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeExists { node_kind, identifier } => {
                if let Some(id) = identifier {
                    write!(f, "NodeExists(kind: {}, id: '{}')", node_kind, id)
                } else {
                    write!(f, "NodeExists(kind: {})", node_kind)
                }
            }
            Self::PropertyEquals { node_id, property, expected_value } => {
                write!(
                    f,
                    "PropertyEquals(node: '{}', prop: '{}' == '{}')",
                    node_id, property, expected_value
                )
            }
            Self::ContainsSubNode { parent_kind, child_kind } => {
                write!(
                    f,
                    "ContainsSubNode(parent: {}, child: {})",
                    parent_kind, child_kind
                )
            }
            Self::DataFlowRelation {
                source_symbol,
                sink_symbol,
                must_pass_sanitizer,
            } => {
                write!(
                    f,
                    "DataFlowRelation(from: '{}', to: '{}', sanitized: {})",
                    source_symbol, sink_symbol, must_pass_sanitizer
                )
            }
            Self::RangeIntersectionEmpty { range_a, range_b } => {
                write!(
                    f,
                    "RangeIntersectionEmpty([{}..{}] ∩ [{}..{}] = ∅)",
                    range_a.0, range_a.1, range_b.0, range_b.1
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Proof Claim
// ---------------------------------------------------------------------------

/// Deterministic proof claim submitted for formal dynamic AST verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofClaim {
    pub claim_id: String,
    pub target_file: String,
    pub target_symbol: String,
    pub invariant_name: String,
    pub assertions: Vec<ProofAssertion>,
    pub created_at_ms: u64,
}

impl ProofClaim {
    /// Creates a new `ProofClaim` without assertions.
    pub fn new(
        claim_id: impl Into<String>,
        target_file: impl Into<String>,
        target_symbol: impl Into<String>,
        invariant_name: impl Into<String>,
        created_at_ms: u64,
    ) -> Self {
        Self {
            claim_id: claim_id.into(),
            target_file: target_file.into(),
            target_symbol: target_symbol.into(),
            invariant_name: invariant_name.into(),
            assertions: Vec::new(),
            created_at_ms,
        }
    }

    /// Builder method to supply a batch of assertions.
    pub fn with_assertions(mut self, assertions: Vec<ProofAssertion>) -> Self {
        self.assertions = assertions;
        self
    }

    /// Appends a single assertion to the claim.
    pub fn add_assertion(&mut self, assertion: ProofAssertion) -> &mut Self {
        self.assertions.push(assertion);
        self
    }

    /// Computes the deterministic SHA-256 digest binding the claim to the source AST hash:
    /// Digest = SHA-256(claim_id || target_symbol || invariant_name || source_ast_hash)
    pub fn compute_claim_hash(&self, source_ast_hash: &str) -> String {
        let payload = format!(
            "{}:{}:{}:{}",
            self.claim_id, self.target_symbol, self.invariant_name, source_ast_hash
        );
        compute_sha256(payload.as_bytes())
    }
}

// ---------------------------------------------------------------------------
// 3. Tri-State Proof Verdict
// ---------------------------------------------------------------------------

/// Deterministic tri-state verdict resulting from dynamic proof verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofVerdict {
    /// The claim and all its constituent assertions were 100% verified and proven.
    Proven,
    /// The claim was refuted due to a concrete assertion violation or contradiction.
    Refuted {
        reason: String,
        failed_assertion_index: usize,
    },
    /// The claim could not be verified due to incomplete context, timeout, or boundary limits.
    Unverifiable {
        reason: String,
    },
}

impl ProofVerdict {
    /// Returns true if the verdict is `Proven`.
    pub fn is_proven(&self) -> bool {
        matches!(self, Self::Proven)
    }

    /// Returns true if the verdict is `Refuted`.
    pub fn is_refuted(&self) -> bool {
        matches!(self, Self::Refuted { .. })
    }

    /// Returns true if the verdict is `Unverifiable`.
    pub fn is_unverifiable(&self) -> bool {
        matches!(self, Self::Unverifiable { .. })
    }

    /// Returns the failure or unverifiable reason, if any.
    pub fn failure_reason(&self) -> Option<&str> {
        match self {
            Self::Proven => None,
            Self::Refuted { reason, .. } => Some(reason.as_str()),
            Self::Unverifiable { reason } => Some(reason.as_str()),
        }
    }

    /// Returns the index of the failed assertion if the verdict is `Refuted`.
    pub fn failed_assertion_index(&self) -> Option<usize> {
        match self {
            Self::Refuted {
                failed_assertion_index,
                ..
            } => Some(*failed_assertion_index),
            _ => None,
        }
    }
}

impl fmt::Display for ProofVerdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proven => write!(f, "PROVEN (100% invariant verified)"),
            Self::Refuted {
                reason,
                failed_assertion_index,
            } => {
                write!(
                    f,
                    "REFUTED at assertion [{}]: {}",
                    failed_assertion_index, reason
                )
            }
            Self::Unverifiable { reason } => write!(f, "UNVERIFIABLE: {}", reason),
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Cryptographic Proof Certificate
// ---------------------------------------------------------------------------

/// Cryptographic certificate proving a `ProofClaim` was verified with 0 AST violations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofCertificate {
    pub certificate_id: String,
    pub claim_id: String,
    pub sha256_signature: String,
    pub verifier_version: String,
    pub issued_at_ms: u64,
    pub execution_latency_ms: f64,
}

impl Eq for ProofCertificate {}

impl ProofCertificate {
    /// Generates a new cryptographic certificate binding the claim, target symbol, invariant, and AST hash.
    pub fn new(
        claim_id: impl Into<String>,
        target_symbol: &str,
        invariant_name: &str,
        source_ast_hash: &str,
        verifier_version: impl Into<String>,
        issued_at_ms: u64,
        execution_latency_ms: f64,
    ) -> Self {
        let claim_id_str = claim_id.into();
        let payload = format!(
            "{}:{}:{}:{}",
            claim_id_str, target_symbol, invariant_name, source_ast_hash
        );
        let digest = compute_sha256(payload.as_bytes());
        let sha256_signature = format!("LOCUS-PROOF-v2:{}", digest);
        let certificate_id = format!("cert-{}", &digest[0..16]);

        Self {
            certificate_id,
            claim_id: claim_id_str,
            sha256_signature,
            verifier_version: verifier_version.into(),
            issued_at_ms,
            execution_latency_ms,
        }
    }

    /// Verifies the cryptographic signature of the certificate against the expected inputs.
    pub fn verify_signature(
        &self,
        target_symbol: &str,
        invariant_name: &str,
        source_ast_hash: &str,
    ) -> bool {
        let payload = format!(
            "{}:{}:{}:{}",
            self.claim_id, target_symbol, invariant_name, source_ast_hash
        );
        let expected_digest = compute_sha256(payload.as_bytes());
        let expected_sig = format!("LOCUS-PROOF-v2:{}", expected_digest);
        self.sha256_signature == expected_sig
    }
}

// ---------------------------------------------------------------------------
// 5. Proof Verification Result
// ---------------------------------------------------------------------------

/// Comprehensive verification result aggregating the claim, verdict, certificate, and evaluation metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProofVerificationResult {
    pub claim: ProofClaim,
    pub verdict: ProofVerdict,
    pub certificate: Option<ProofCertificate>,
    pub steps_evaluated: usize,
}

impl ProofVerificationResult {
    /// Constructs a successful `Proven` verification result with its cryptographic certificate.
    pub fn proven(
        claim: ProofClaim,
        certificate: ProofCertificate,
        steps_evaluated: usize,
    ) -> Self {
        Self {
            claim,
            verdict: ProofVerdict::Proven,
            certificate: Some(certificate),
            steps_evaluated,
        }
    }

    /// Constructs a `Refuted` verification result.
    pub fn refuted(
        claim: ProofClaim,
        reason: impl Into<String>,
        failed_assertion_index: usize,
        steps_evaluated: usize,
    ) -> Self {
        Self {
            claim,
            verdict: ProofVerdict::Refuted {
                reason: reason.into(),
                failed_assertion_index,
            },
            certificate: None,
            steps_evaluated,
        }
    }

    /// Constructs an `Unverifiable` verification result.
    pub fn unverifiable(
        claim: ProofClaim,
        reason: impl Into<String>,
        steps_evaluated: usize,
    ) -> Self {
        Self {
            claim,
            verdict: ProofVerdict::Unverifiable {
                reason: reason.into(),
            },
            certificate: None,
            steps_evaluated,
        }
    }

    /// Returns true if the result was proven and a valid certificate was issued.
    pub fn is_proven(&self) -> bool {
        self.verdict.is_proven() && self.certificate.is_some()
    }
}

// ---------------------------------------------------------------------------
// 6. Pure Safe Rust SHA-256 Implementation (FIPS 180-4)
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

/// Computes standard cryptographic SHA-256 hash in pure 100% safe Rust.
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
            *w_elem = u32::from_be_bytes([
                chunk[start],
                chunk[start + 1],
                chunk[start + 2],
                chunk[start + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
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
            let temp1 = h_var
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
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
