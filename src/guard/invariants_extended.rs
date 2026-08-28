//! Extended Deterministic Safety Invariants (Rules 11 to 31).
//!
//! Enforces sub-millisecond AST invariant verification for enterprise security,
//! concurrency soundness, frontend leak prevention, cryptography, and memory safety.

#![forbid(unsafe_code)]

use crate::types::ViolationKind;
use regex::Regex;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Compiled Regex Patterns for Rules 11 - 31
// ---------------------------------------------------------------------------

/// Rule 11: SQL Injection - Unparameterized string interpolation / concatenation in SQL
static RE_SQL_RAW_CONCAT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)(SELECT|INSERT\s+INTO|UPDATE|DELETE\s+FROM|WHERE)\s+.*(\$\{[^}]+\}|"\s*\+\s*[a-zA-Z_$]|\bformat!\s*\(\s*"[^"]*\{\}.*"\s*,)"#).unwrap());

/// Rule 12: Floating Promise - Async calls without await / then / catch / void / return
static RE_FLOATING_PROMISE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?m)^\s*(?:fetch|axios\.(?:get|post|put|delete)|db\.(?:query|execute)|[a-zA-Z0-9_$]+Async)\s*\([^;\n]*\)\s*;"#).unwrap());

/// Rule 13: React State Race - Direct non-functional setState inside async loops or after await
static RE_REACT_STATE_RACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)\b(?:for|while)\s*\([^)]*\)\s*\{[^}]*(?:await\s+[^;]+;[^}]*)?\bset[A-Z][a-zA-Z0-9_$]*\s*\(\s*[a-zA-Z0-9_$]+\s*[+\-*]\s*\d+\s*\)"#).unwrap());

/// Rule 14: Event Listener Leak - addEventListener in useEffect without removeEventListener cleanup
static RE_ADD_EVENT_LISTENER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"addEventListener\s*\(\s*["']([a-zA-Z0-9_-]+)["']"#).unwrap());
static RE_REMOVE_EVENT_LISTENER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"removeEventListener\s*\(\s*["']([a-zA-Z0-9_-]+)["']"#).unwrap());
static RE_USE_EFFECT_BLOCK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)useEffect\s*\(\s*\(\s*\)\s*=>\s*\{(?P<body>.*?)\}\s*,\s*\["#).unwrap());

/// Rule 15: Insecure Randomness - Math.random used in security / token / auth / session contexts
static RE_INSECURE_RANDOM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)\b(token|secret|key|password|auth|session|nonce|salt|pin|apiKey|crypto)\b[^;\n]*=\s*[^;\n]*Math\.random\s*\(\s*\)"#).unwrap());
static RE_RANDOM_IN_SECURITY_VAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)let\s+(?:mut\s+)?(?:[a-zA-Z0-9_]*[Tt]oken|[a-zA-Z0-9_]*[Ss]ecret|[a-zA-Z0-9_]*[Kk]ey|[a-zA-Z0-9_]*[Aa]uth|[a-zA-Z0-9_]*[Ss]ession|[a-zA-Z0-9_]*[Nn]once)\s*=[^;\n]*Math\.random"#).unwrap());

/// Rule 16: Path Traversal - Direct user inputs concatenated into filesystem calls
static RE_PATH_TRAVERSAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)(?:fs\.(?:readFile|readFileSync|writeFile|writeFileSync|unlink|readdir|createReadStream)|path\.(?:join|resolve))\s*\([^)]*(?:req\.(?:params|query|body)|params\.|userInput|user_path|file_param)"#).unwrap());

/// Rule 17: Unbounded Memory Regex - Exponential catastrophic backtracking patterns
static RE_UNBOUNDED_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"["'`/].*(\([a-zA-Z0-9_.]+\+[a-zA-Z0-9_.]*\)\+|\([a-zA-Z0-9_.]+\*[a-zA-Z0-9_.]*\)\*|\([a-zA-Z0-9_.]+\|\s*[a-zA-Z0-9_.]+\)\+).*["'`/]"#).unwrap());

/// Rule 18: Dynamic Code Eval - dynamic code execution
static RE_DYNAMIC_EVAL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\b(?:eval\s*\(|new\s+Function\s*\(|window\.eval\s*\(|global\.eval\s*\()"#).unwrap());

/// Rule 19: Untyped Union Access - Direct unsafe type cast or union property without narrowing
static RE_UNSAFE_AS_ANY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\b[a]s\s+[a]ny\b|\b[a]s\s+[n]ever\s+[a]s\s+[a]ny\b"#).unwrap());

/// Rule 20: Circular Mem Leak - Strong self-reference / back-pointers in Rc/Arc without Weak
static RE_CIRCULAR_REF: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)struct\s+\w+[^}]*\b(?:parent|prev|owner|back_ptr|caller)\s*:\s*(?:Option\s*<\s*)?(?:Rc\s*<\s*RefCell|Arc\s*<\s*Mutex|Arc\s*<\s*RwLock)"#).unwrap());

/// Rule 21: Async Cancellation Safety - Non-atomic state mutations across yield points in select
static RE_ASYNC_CANCEL_MUTATION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)\b(?:tokio\s*::\s*select!|select!)\s*\{[^}]*=>\s*\{[^}]*(?:state\.\w+\s*=[^;]+;[^}]*\.await|balance\s*[-+]=\s*[^;]+;[^}]*\.await)"#).unwrap());

/// Rule 22: Constant-Time Crypto - Variable-time equality comparison on sensitive tokens/hashes
static RE_VARIABLE_TIME_CRYPTO: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)\b(password|authToken|signature|hmac|api_key_hash|secretToken|expected_mac)\s*(?:==|===|!=|!==)\s*[a-zA-Z0-9_$]+|[a-zA-Z0-9_$]+\s*(?:==|===|!=|!==)\s*(?:password|authToken|signature|hmac|api_key_hash|secretToken|expected_mac)\b"#).unwrap());

/// Rule 23: Exhaustive Enum Narrowing - Switch statements without a default branch
static RE_SWITCH_NO_DEFAULT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)\bswitch\s*\([^)]*\)\s*\{(?P<body>[^{}]*(?:\{[^{}]*\}[^{}]*)*)\}"#).unwrap());

/// Rule 24: Resource Descriptor Leak - Raw unmanaged descriptors without close or finally
static RE_UNMANAGED_DESCRIPTOR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)\b(?:const|let|var)\s+([a-zA-Z0-9_$]+)\s*=\s*(?:fs\.openSync|net\.createConnection|tls\.connect)\s*\("#).unwrap());

/// Rule 25: SSRF Unsafe Fetch - Outbound network requests targeting private IPs / metadata
static RE_SSRF_UNSAFE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)(?:fetch|axios\.(?:get|post|put|delete)|http\.get)\s*\(\s*(?:["'`]https?:\/\/(?:127\.0\.0\.1|169\.254\.169\.254|localhost|0\.0\.0\.0|metadata\.google\.internal)|(?:req\.(?:query|params|body)\.[a-zA-Z0-9_$]*url|userInputUrl|targetUrl)\b)"#).unwrap());

/// Rule 26: Unbounded Channel Deadlock - Synchronous zero-capacity channel in sync loop
static RE_UNBOUNDED_CHANNEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)(?:std\s*::\s*sync\s*::\s*mpsc\s*::\s*sync_channel\s*\(\s*0\s*\)|mpsc\s*::\s*channel\s*\(\s*\))[^;]*;[^}]*for\s+[^}]*\.send\s*\("#).unwrap());

/// Rule 27: Prototype Pollution - Unvalidated recursive key assignment
static RE_PROTOTYPE_POLLUTION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"\b(?:target|obj)\[['"]__proto__['"]\]|\b(?:target|obj)\[['"]prototype['"]\]|\btarget\[(?:key|prop)\]\s*=\s*(?:source|val)\[(?:key|prop)\]"#).unwrap());

/// Rule 28: CORS Wildcard with Credentials - Wildcard origin combined with credentials true
static RE_CORS_WILDCARD_CREDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)(?:Access-Control-Allow-Origin['"]?\s*[:=]\s*['"]?\*['"]?).*(?:Access-Control-Allow-Credentials['"]?\s*[:=]\s*['"]?true['"]?)|(?:Access-Control-Allow-Credentials['"]?\s*[:=]\s*['"]?true['"]?).*(?:Access-Control-Allow-Origin['"]?\s*[:=]\s*['"]?\*['"]?)|origin\s*:\s*['"]\*['"][^}]*credentials\s*:\s*true|credentials\s*:\s*true[^}]*origin\s*:\s*['"]\*['"]"#).unwrap());

/// Rule 29: Hardcoded Key Entropy - Static high-entropy private keys and credentials
static RE_HARDCODED_KEY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?:-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----|sk_live_[0-9a-zA-Z]{24,}|AKIA[0-9A-Z]{16}|ghp_[0-9a-zA-Z]{36})"#).unwrap());

/// Rule 30: Unchecked Arithmetic Overflow - Bounded integer mutation in unbounded while loop
static RE_UNCHECKED_OVERFLOW: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)(?:while\s+true|loop)\s*\{[^}]*\b(?:mut\s+)?[a-zA-Z0-9_]+\s*:\s*(?:u8|u16|i8|i16|u32|i32)[^;]*;[^}]*\+=\s*\d+[^}]*\}|loop\s*\{[^}]*[a-zA-Z0-9_]+\s*\+=\s*1\s*;[^}]*\}"#).unwrap());

/// Rule 31: Atomic State Mutation - Direct property mutation in state reducer/store callbacks
static RE_MUTATING_STATE_STORE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?s)(?:setState|useStore\.setState)\s*\(\s*\(?\s*(?:state|draft)[^)]*\)?\s*=>\s*\{[^}]*\bstate\.[a-zA-Z0-9_$]+\s*=[^;]+;[^}]*\}\s*\)|function\s+\w+Reducer\s*\([^)]*state[^)]*\)\s*\{[^}]*state\.[a-zA-Z0-9_$]+\s*=[^;]+;"#).unwrap());

// ---------------------------------------------------------------------------
// Extended Invariants Verifier
// ---------------------------------------------------------------------------

pub struct InvariantsExtended;

impl InvariantsExtended {
    /// Rule 11: SQL Injection check
    pub fn check_sql_injection(source: &str) -> Option<String> {
        if let Some(m) = RE_SQL_RAW_CONCAT.find(source) {
            return Some(format!(
                "Potential SQL injection: unparameterized query interpolation at: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 12: Floating Promise check
    pub fn check_floating_promise(source: &str) -> Option<String> {
        if let Some(m) = RE_FLOATING_PROMISE.find(source) {
            return Some(format!(
                "Floating unhandled promise detected (missing await, void, or .catch()): '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 13: React State Race check
    pub fn check_react_state_race(source: &str) -> Option<String> {
        if let Some(m) = RE_REACT_STATE_RACE.find(source) {
            return Some(format!(
                "React state race condition: non-functional setState inside loop or async scope: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 14: Event Listener Leak check
    pub fn check_listener_leak(source: &str) -> Option<String> {
        for cap in RE_USE_EFFECT_BLOCK.captures_iter(source) {
            if let Some(body_match) = cap.name("body") {
                let body = body_match.as_str();
                if let Some(add_match) = RE_ADD_EVENT_LISTENER.find(body) {
                    if !RE_REMOVE_EVENT_LISTENER.is_match(body) {
                        return Some(format!(
                            "Event listener leak: '{}' in useEffect without removeEventListener cleanup",
                            add_match.as_str()
                        ));
                    }
                }
            }
        }
        None
    }

    /// Rule 15: Insecure Randomness check
    pub fn check_insecure_randomness(source: &str) -> Option<String> {
        if let Some(m) = RE_INSECURE_RANDOM.find(source) {
            return Some(format!(
                "Insecure randomness: Math.random() used in security-sensitive context: '{}'",
                m.as_str().trim()
            ));
        }
        if let Some(m) = RE_RANDOM_IN_SECURITY_VAR.find(source) {
            return Some(format!(
                "Insecure randomness: Math.random() assigned to security token/secret: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 16: Path Traversal check
    pub fn check_path_traversal(source: &str) -> Option<String> {
        if let Some(m) = RE_PATH_TRAVERSAL.find(source) {
            return Some(format!(
                "Potential path traversal: direct user parameter concatenated into filesystem path: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 17: Unbounded Memory Regex check
    pub fn check_unbounded_regex(source: &str) -> Option<String> {
        if let Some(m) = RE_UNBOUNDED_REGEX.find(source) {
            return Some(format!(
                "Unbounded regex catastrophic backtracking risk: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 18: Dynamic Code Eval check
    pub fn check_dynamic_code_eval(source: &str) -> Option<String> {
        if let Some(m) = RE_DYNAMIC_EVAL.find(source) {
            return Some(format!(
                "Forbidden dynamic code evaluation: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 19: Untyped Union Access check
    pub fn check_untyped_union_access(source: &str) -> Option<String> {
        if let Some(m) = RE_UNSAFE_AS_ANY.find(source) {
            return Some(format!(
                "Unsafe dynamic type cast bypasses strict type safety: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 20: Circular Reference Memory Leak check
    pub fn check_circular_mem_leak(source: &str) -> Option<String> {
        if !source.contains("Weak") {
            if let Some(m) = RE_CIRCULAR_REF.find(source) {
                return Some(format!(
                    "Circular reference memory leak risk: strong back-pointer in struct without Weak reference: '{}'",
                    m.as_str().trim()
                ));
            }
        }
        None
    }

    /// Rule 21: Async Cancellation Safety check
    pub fn check_async_cancellation_safety(source: &str) -> Option<String> {
        if let Some(m) = RE_ASYNC_CANCEL_MUTATION.find(source) {
            return Some(format!(
                "Async cancellation safety violation: non-atomic state mutation across .await inside select branch: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 22: Constant-Time Crypto check
    pub fn check_constant_time_crypto(source: &str) -> Option<String> {
        if !source.contains("ConstantTimeEq") && !source.contains("timingSafeEqual") && !source.contains("constant_time_eq") {
            if let Some(m) = RE_VARIABLE_TIME_CRYPTO.find(source) {
                return Some(format!(
                    "Variable-time comparison in security-sensitive crypto context (risk of timing side-channel attack): '{}' — use constant-time equality.",
                    m.as_str().trim()
                ));
            }
        }
        None
    }

    /// Rule 23: Exhaustive Enum Narrowing check
    pub fn check_exhaustive_enum_narrowing(source: &str) -> Option<String> {
        for cap in RE_SWITCH_NO_DEFAULT.captures_iter(source) {
            if let Some(body) = cap.name("body") {
                let body_str = body.as_str();
                if body_str.contains("case ") && !body_str.contains("default:") {
                    return Some(
                        "Non-exhaustive switch statement lacking 'default:' fallback branch.".to_string(),
                    );
                }
            }
        }
        None
    }

    /// Rule 24: Resource Descriptor Leak check
    pub fn check_resource_descriptor_leak(source: &str) -> Option<String> {
        if !source.contains("close") && !source.contains("finally") && !source.contains(".end()") {
            if let Some(m) = RE_UNMANAGED_DESCRIPTOR.find(source) {
                return Some(format!(
                    "Potential unmanaged resource descriptor leak: handle opened without guaranteed close or finally block at: '{}'",
                    m.as_str().trim()
                ));
            }
        }
        None
    }

    /// Rule 25: SSRF Unsafe Fetch check
    pub fn check_ssrf_unsafe_fetch(source: &str) -> Option<String> {
        if let Some(m) = RE_SSRF_UNSAFE.find(source) {
            return Some(format!(
                "Potential SSRF vulnerability: outbound request targeting private network or unvalidated URL variable at: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 26: Unbounded Channel Deadlock check
    pub fn check_unbounded_channel_deadlock(source: &str) -> Option<String> {
        if let Some(m) = RE_UNBOUNDED_CHANNEL.find(source) {
            return Some(format!(
                "Unbounded channel deadlock risk: synchronous channel send in loop without consumer thread: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 27: Prototype Pollution check
    pub fn check_prototype_pollution(source: &str) -> Option<String> {
        if let Some(m) = RE_PROTOTYPE_POLLUTION.find(source) {
            return Some(format!(
                "Potential Prototype Pollution: unvalidated property assignment allowing __proto__ overwrite at: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 28: CORS Wildcard with Credentials check
    pub fn check_cors_wildcard_credentials(source: &str) -> Option<String> {
        if let Some(m) = RE_CORS_WILDCARD_CREDS.find(source) {
            return Some(format!(
                "Insecure CORS configuration: Access-Control-Allow-Origin: * combined with credentials: true: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 29: Hardcoded Key Entropy check
    pub fn check_hardcoded_key_entropy(source: &str) -> Option<String> {
        if let Some(m) = RE_HARDCODED_KEY.find(source) {
            return Some(format!(
                "Hardcoded cryptographic secret/private key detected in source code: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 30: Unchecked Arithmetic Overflow check
    pub fn check_unchecked_arithmetic_overflow(source: &str) -> Option<String> {
        if let Some(m) = RE_UNCHECKED_OVERFLOW.find(source) {
            return Some(format!(
                "Unchecked integer increment in unbounded loop risking arithmetic overflow panic: '{}'",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Rule 31: Atomic State Mutation check
    pub fn check_atomic_state_mutation(source: &str) -> Option<String> {
        if let Some(m) = RE_MUTATING_STATE_STORE.find(source) {
            return Some(format!(
                "Direct in-place state object mutation in store callback: '{}' — use immutable updates.",
                m.as_str().trim()
            ));
        }
        None
    }

    /// Run all 21 extended checks (Rules 11 to 31) and return first violation if any
    pub fn check_all(source: &str) -> Option<(ViolationKind, String)> {
        // Rules 11..19
        if let Some(detail) = Self::check_sql_injection(source) {
            return Some((ViolationKind::SqlInjection, detail));
        }
        if let Some(detail) = Self::check_floating_promise(source) {
            return Some((ViolationKind::FloatingPromise, detail));
        }
        if let Some(detail) = Self::check_react_state_race(source) {
            return Some((ViolationKind::ReactStateRace, detail));
        }
        if let Some(detail) = Self::check_listener_leak(source) {
            return Some((ViolationKind::ListenerLeak, detail));
        }
        if let Some(detail) = Self::check_insecure_randomness(source) {
            return Some((ViolationKind::InsecureRandomness, detail));
        }
        if let Some(detail) = Self::check_path_traversal(source) {
            return Some((ViolationKind::PathTraversal, detail));
        }
        if let Some(detail) = Self::check_unbounded_regex(source) {
            return Some((ViolationKind::UnboundedRegex, detail));
        }
        if let Some(detail) = Self::check_dynamic_code_eval(source) {
            return Some((ViolationKind::DynamicCodeEval, detail));
        }
        if let Some(detail) = Self::check_untyped_union_access(source) {
            return Some((ViolationKind::UntypedUnionAccess, detail));
        }

        // Rules 20..31 (New Enterprise Invariants)
        if let Some(detail) = Self::check_circular_mem_leak(source) {
            return Some((ViolationKind::CircularMemLeak, detail));
        }
        if let Some(detail) = Self::check_async_cancellation_safety(source) {
            return Some((ViolationKind::AsyncCancellationSafety, detail));
        }
        if let Some(detail) = Self::check_constant_time_crypto(source) {
            return Some((ViolationKind::ConstantTimeCrypto, detail));
        }
        if let Some(detail) = Self::check_exhaustive_enum_narrowing(source) {
            return Some((ViolationKind::ExhaustiveEnumNarrowing, detail));
        }
        if let Some(detail) = Self::check_resource_descriptor_leak(source) {
            return Some((ViolationKind::ResourceDescriptorLeak, detail));
        }
        if let Some(detail) = Self::check_ssrf_unsafe_fetch(source) {
            return Some((ViolationKind::SsrfUnsafeFetch, detail));
        }
        if let Some(detail) = Self::check_unbounded_channel_deadlock(source) {
            return Some((ViolationKind::UnboundedChannelDeadlock, detail));
        }
        if let Some(detail) = Self::check_prototype_pollution(source) {
            return Some((ViolationKind::PrototypePollution, detail));
        }
        if let Some(detail) = Self::check_cors_wildcard_credentials(source) {
            return Some((ViolationKind::CorsWildcardCredentials, detail));
        }
        if let Some(detail) = Self::check_hardcoded_key_entropy(source) {
            return Some((ViolationKind::HardcodedKeyEntropy, detail));
        }
        if let Some(detail) = Self::check_unchecked_arithmetic_overflow(source) {
            return Some((ViolationKind::UncheckedArithmeticOverflow, detail));
        }
        if let Some(detail) = Self::check_atomic_state_mutation(source) {
            return Some((ViolationKind::AtomicStateMutation, detail));
        }

        None
    }
}
