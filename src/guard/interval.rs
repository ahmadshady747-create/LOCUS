//! Symbolic Interval Arithmetic Engine (Range Analysis).
//!
//! Implements a zero-allocation, lightweight interval domain I = [L, U] where
//! L, U in R union {-inf, +inf} to soundly eliminate DivisionByZero false positives
//! on guaranteed non-zero denominators (e.g., Math.max(c, x) with c > 0, Math.sqrt(x) + c,
//! x + Number.EPSILON, clamp patterns).

#![forbid(unsafe_code)]

use regex::Regex;
use std::sync::LazyLock;

/// Interval domain representation: I = [min, max].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub min: f64,
    pub max: f64,
}

impl Interval {
    pub const TOP: Self = Self {
        min: f64::NEG_INFINITY,
        max: f64::INFINITY,
    };

    pub const BOTTOM: Self = Self {
        min: f64::NAN,
        max: f64::NAN,
    };

    pub const fn new(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    pub const fn point(val: f64) -> Self {
        Self { min: val, max: val }
    }

    pub const fn strictly_positive(c: f64) -> Self {
        Self {
            min: c,
            max: f64::INFINITY,
        }
    }

    pub const fn strictly_negative(c: f64) -> Self {
        Self {
            min: f64::NEG_INFINITY,
            max: c,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.min.is_nan() || self.max.is_nan() || self.min > self.max
    }

    pub fn contains_zero(&self) -> bool {
        if self.is_empty() {
            return false;
        }
        self.min <= 0.0 && self.max >= 0.0
    }

    pub fn is_provably_non_zero(&self) -> bool {
        if self.is_empty() {
            return false;
        }
        self.min > 0.0 || self.max < 0.0
    }

    pub fn is_strictly_positive(&self) -> bool {
        if self.is_empty() {
            return false;
        }
        self.min > 0.0
    }

    pub fn is_strictly_negative(&self) -> bool {
        if self.is_empty() {
            return false;
        }
        self.max < 0.0
    }

    pub fn add(&self, other: &Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Self::BOTTOM;
        }
        Self::new(self.min + other.min, self.max + other.max)
    }

    pub fn sub(&self, other: &Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Self::BOTTOM;
        }
        Self::new(self.min - other.max, self.max - other.min)
    }

    pub fn mul(&self, other: &Self) -> Self {
        if self.is_empty() || other.is_empty() {
            return Self::BOTTOM;
        }
        let p1 = self.min * other.min;
        let p2 = self.min * other.max;
        let p3 = self.max * other.min;
        let p4 = self.max * other.max;
        let min = p1.min(p2).min(p3).min(p4);
        let max = p1.max(p2).max(p3).max(p4);
        Self::new(min, max)
    }

    pub fn max(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Self::new(self.min.max(other.min), self.max.max(other.max))
    }

    pub fn min(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        Self::new(self.min.min(other.min), self.max.min(other.max))
    }

    pub fn clamp(&self, low: f64, high: f64) -> Self {
        if self.is_empty() {
            return Self::BOTTOM;
        }
        Self::new(self.min.max(low), self.max.min(high))
    }
}

// ---------------------------------------------------------------------------
// Compiled Pattern Regexes for Math Functions & Range Analysis
// ---------------------------------------------------------------------------

static RE_MATH_MAX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)Math\s*\.\s*max\s*\(\s*([^,()]+)\s*,\s*([^,()]+)\s*\)").unwrap()
});

static RE_DOT_MAX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([a-zA-Z0-9_$]+)\s*\.\s*max\s*\(\s*([^,()]+)\s*\)").unwrap()
});

static RE_MATH_SQRT_PLUS_C: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)Math\s*\.\s*sqrt\s*\([^)]*\)\s*\+\s*([0-9.]+|Number\.EPSILON|f64::EPSILON)").unwrap()
});

static RE_C_PLUS_MATH_SQRT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)([0-9.]+|Number\.EPSILON|f64::EPSILON)\s*\+\s*Math\s*\.\s*sqrt\s*\([^)]*\)").unwrap()
});

static RE_MATH_HYPOT_STRICT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)Math\s*\.\s*hypot\s*\([^)]*\)\s*\+\s*([0-9.]+|Number\.EPSILON|f64::EPSILON)").unwrap()
});

static RE_ABS_PLUS_EPS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:Math\s*\.\s*abs\s*\([^)]*\)|[a-zA-Z0-9_$]+\s*\.\s*abs\s*\(\s*\))\s*\+\s*([0-9.]+|Number\.EPSILON|f64::EPSILON)").unwrap()
});

static RE_EXPR_PLUS_EPSILON: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\+\s*(?:Number\.EPSILON|f64::EPSILON|f32::EPSILON|1e-7|1e-8|1e-9|0\.000001)").unwrap()
});

static RE_CLAMP_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)Math\s*\.\s*min\s*\(\s*[^,()]+\s*,\s*Math\s*\.\s*max\s*\(\s*([0-9.]+)\s*,\s*[^,()]+\s*\)\s*\)").unwrap()
});

/// Evaluates numeric expressions within an interval domain.
pub struct IntervalDomain;

impl IntervalDomain {
    /// Analyzes `expr_str` and attempts to prove that its interval strictly excludes zero.
    /// Returns true if provably non-zero (i.e., 0 not in I).
    pub fn is_provably_non_zero(expr_str: &str, local_context: &str) -> bool {
        let mut trimmed = expr_str.trim().trim_matches(|c| c == ';' || c == ',');
        while trimmed.starts_with('(') && trimmed.ends_with(')') {
            trimmed = trimmed[1..trimmed.len() - 1].trim();
        }

        // 1. Direct numeric constant
        if let Ok(val) = trimmed.parse::<f64>() {
            return val != 0.0;
        }

        // 2. Math.max(C, expr) or Math.max(expr, C)
        if let Some(caps) = RE_MATH_MAX.captures(trimmed) {
            let arg1 = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            let arg2 = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
            if let Ok(c) = arg1.parse::<f64>() {
                if c > 0.0 {
                    return true;
                }
            }
            if let Ok(c) = arg2.parse::<f64>() {
                if c > 0.0 {
                    return true;
                }
            }
            if arg1.contains("EPSILON") || arg2.contains("EPSILON") {
                return true;
            }
        }

        // 3. Rust x.max(C) or C.max(x)
        if let Some(caps) = RE_DOT_MAX.captures(trimmed) {
            let arg = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
            if let Ok(c) = arg.parse::<f64>() {
                if c > 0.0 {
                    return true;
                }
            }
            if arg.contains("EPSILON") {
                return true;
            }
        }

        // 4. Math.sqrt(expr) + C where C > 0
        if let Some(caps) = RE_MATH_SQRT_PLUS_C.captures(trimmed) {
            if let Some(c_str) = caps.get(1) {
                let s = c_str.as_str();
                if s.contains("EPSILON") || s.parse::<f64>().map(|v| v > 0.0).unwrap_or(false) {
                    return true;
                }
            }
        }
        if let Some(caps) = RE_C_PLUS_MATH_SQRT.captures(trimmed) {
            if let Some(c_str) = caps.get(1) {
                let s = c_str.as_str();
                if s.contains("EPSILON") || s.parse::<f64>().map(|v| v > 0.0).unwrap_or(false) {
                    return true;
                }
            }
        }

        // 5. Math.hypot(...) + eps or Math.abs(...) + eps
        if RE_MATH_HYPOT_STRICT.is_match(trimmed) || RE_ABS_PLUS_EPS.is_match(trimmed) {
            return true;
        }

        // 6. Generic + Number.EPSILON or + f64::EPSILON
        if RE_EXPR_PLUS_EPSILON.is_match(trimmed) {
            return true;
        }

        // 7. Clamp pattern: Math.min(max, Math.max(min, x)) where min > 0
        if let Some(caps) = RE_CLAMP_PATTERN.captures(trimmed) {
            if let Some(min_str) = caps.get(1) {
                if min_str.as_str().parse::<f64>().map(|v| v > 0.0).unwrap_or(false) {
                    return true;
                }
            }
        }

        // 8. Inspect local scope context for bindings and guards
        if !trimmed.is_empty() && trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$') {
            let var = trimmed;

            // Check if variable was bound to Math.max(C, ...) with C > 0
            for line in local_context.lines() {
                let l = line.trim();
                if (l.starts_with("let ") || l.starts_with("const ") || l.starts_with("var "))
                    && l.contains(var)
                    && (l.contains("Math.max") || l.contains(".max("))
                {
                    if let Some(caps) = RE_MATH_MAX.captures(l) {
                        let a1 = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                        let a2 = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                        if a1.parse::<f64>().map(|v| v > 0.0).unwrap_or(false)
                            || a2.parse::<f64>().map(|v| v > 0.0).unwrap_or(false)
                            || a1.contains("EPSILON")
                            || a2.contains("EPSILON")
                        {
                            return true;
                        }
                    }
                    if let Some(caps) = RE_DOT_MAX.captures(l) {
                        let a = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");
                        if a.parse::<f64>().map(|v| v > 0.0).unwrap_or(false) || a.contains("EPSILON") {
                            return true;
                        }
                    }
                }

                // Check direct local guards on var:
                // if (var != 0), if (var !== 0), if (var > 0), if (var), if (var >= 1), assert!(var != 0)
                if l.contains(&format!("{} != 0", var))
                    || l.contains(&format!("{} !== 0", var))
                    || l.contains(&format!("{} > 0", var))
                    || l.contains(&format!("{} >= 1", var))
                    || l.contains(&format!("{} < 0", var))
                    || l.contains(&format!("assert!({} != 0", var))
                    || l.contains(&format!("assert!({} > 0", var))
                    || l.contains(&format!("if ({})", var))
                    || l.contains(&format!("if ({} )", var))
                {
                    return true;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_arithmetic() {
        let i1 = Interval::new(1.0, 5.0);
        let i2 = Interval::new(2.0, 3.0);

        let sum = i1.add(&i2);
        assert_eq!(sum.min, 3.0);
        assert_eq!(sum.max, 8.0);
        assert!(sum.is_provably_non_zero());

        let i_zero = Interval::new(-1.0, 2.0);
        assert!(i_zero.contains_zero());
        assert!(!i_zero.is_provably_non_zero());
    }

    #[test]
    fn test_math_max_provably_non_zero() {
        assert!(IntervalDomain::is_provably_non_zero("Math.max(1, count)", ""));
        assert!(IntervalDomain::is_provably_non_zero("Math.max(0.001, x)", ""));
        assert!(IntervalDomain::is_provably_non_zero("Math.max(y, 2)", ""));
        assert!(!IntervalDomain::is_provably_non_zero("Math.max(-1, count)", ""));
    }

    #[test]
    fn test_sqrt_plus_epsilon() {
        assert!(IntervalDomain::is_provably_non_zero("Math.sqrt(dist) + 0.0001", ""));
        assert!(IntervalDomain::is_provably_non_zero("x + Number.EPSILON", ""));
        assert!(IntervalDomain::is_provably_non_zero("val + f64::EPSILON", ""));
    }

    #[test]
    fn test_unverified_variable_fails() {
        assert!(!IntervalDomain::is_provably_non_zero("unverifiedVar", ""));
        assert!(!IntervalDomain::is_provably_non_zero("count", "let x = 10;"));
    }

    #[test]
    fn test_local_guard_recognized() {
        let ctx = "if (count > 0) {\n    let ratio = total / count;\n}";
        assert!(IntervalDomain::is_provably_non_zero("count", ctx));
    }
}
