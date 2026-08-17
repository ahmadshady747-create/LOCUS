use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Default maximum number of files stored in the AST cache before eviction kicks in
pub const DEFAULT_MAX_CACHE_ENTRIES: usize = 10_000;

/// A cached AST code symbol with pre-computed vector embedding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSymbol {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub snippet: String,
    pub doc_id: String,
    pub vector: Vec<f32>,
}

/// Analysis and symbol extraction result for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysisEntry {
    pub file_path: String,
    pub content_hash: String,
    pub symbols: Vec<CachedSymbol>,
    pub token_count: usize,
    pub extracted_at: DateTime<Utc>,
    pub file_size_bytes: usize,
    pub last_accessed: DateTime<Utc>,
}

impl FileAnalysisEntry {
    pub fn new(
        file_path: impl Into<String>,
        content_hash: impl Into<String>,
        symbols: Vec<CachedSymbol>,
        token_count: usize,
        file_size_bytes: usize,
    ) -> Self {
        let now = Utc::now();
        Self {
            file_path: file_path.into(),
            content_hash: content_hash.into(),
            symbols,
            token_count,
            extracted_at: now,
            file_size_bytes,
            last_accessed: now,
        }
    }
}

/// Real-time cache performance and memory metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub cached_files_count: usize,
    pub total_cached_symbols: usize,
    pub saved_embeddings_count: usize,
    pub max_capacity: usize,
    pub hit_rate_percent: f64,
}

/// Thread-safe in-memory cache for AST analysis, extracted symbols, and embeddings
pub struct AstContextCache {
    entries: Arc<RwLock<HashMap<String, FileAnalysisEntry>>>,
    max_capacity: usize,
    hits: AtomicU64,
    misses: AtomicU64,
    saved_embeddings: AtomicU64,
}

impl AstContextCache {
    /// Creates a new AST cache with the default maximum capacity (10,000 files)
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_MAX_CACHE_ENTRIES)
    }

    /// Creates an AST cache with a custom maximum capacity
    pub fn with_capacity(max_capacity: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_capacity: max_capacity.max(1),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            saved_embeddings: AtomicU64::new(0),
        }
    }

    /// Computes the standard SHA-256 hash of a string's UTF-8 bytes, returning a 64-char hex string
    pub fn compute_content_hash(text: &str) -> String {
        Self::compute_sha256(text.as_bytes())
    }

    /// Pure Rust FIPS 180-4 standard SHA-256 implementation
    pub fn compute_sha256(data: &[u8]) -> String {
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

        let mut h0: u32 = 0x6a09e667;
        let mut h1: u32 = 0xbb67ae85;
        let mut h2: u32 = 0x3c6ef372;
        let mut h3: u32 = 0xa54ff53a;
        let mut h4: u32 = 0x510e527f;
        let mut h5: u32 = 0x9b05688c;
        let mut h6: u32 = 0x1f83d9ab;
        let mut h7: u32 = 0x5be0cd19;

        let len = data.len();
        let bit_len = (len as u64) * 8;

        let mut padded = data.to_vec();
        padded.push(0x80);

        while (padded.len() + 8) % 64 != 0 {
            padded.push(0x00);
        }

        padded.extend_from_slice(&bit_len.to_be_bytes());

        for chunk in padded.chunks_exact(64) {
            let mut w = [0u32; 64];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    chunk[i * 4],
                    chunk[i * 4 + 1],
                    chunk[i * 4 + 2],
                    chunk[i * 4 + 3],
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

            let mut a = h0;
            let mut b = h1;
            let mut c = h2;
            let mut d = h3;
            let mut e = h4;
            let mut f = h5;
            let mut g = h6;
            let mut h = h7;

            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let temp1 = h
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(maj);

                h = g;
                g = f;
                f = e;
                e = d.wrapping_add(temp1);
                d = c;
                c = b;
                b = a;
                a = temp1.wrapping_add(temp2);
            }

            h0 = h0.wrapping_add(a);
            h1 = h1.wrapping_add(b);
            h2 = h2.wrapping_add(c);
            h3 = h3.wrapping_add(d);
            h4 = h4.wrapping_add(e);
            h5 = h5.wrapping_add(f);
            h6 = h6.wrapping_add(g);
            h7 = h7.wrapping_add(h);
        }

        format!(
            "{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}{:08x}",
            h0, h1, h2, h3, h4, h5, h6, h7
        )
    }

    /// Checks if a valid cached analysis entry exists for the given file path and current content.
    /// If valid, updates access timestamp, records a hit, and returns the cached entry.
    pub fn get_valid_entry(&self, file_path: &str, current_content: &str) -> Option<FileAnalysisEntry> {
        let current_hash = Self::compute_content_hash(current_content);
        let mut lock = self.entries.write();

        if let Some(entry) = lock.get_mut(file_path) {
            if entry.content_hash == current_hash {
                entry.last_accessed = Utc::now();
                self.hits.fetch_add(1, Ordering::Relaxed);
                self.saved_embeddings
                    .fetch_add(entry.symbols.len().max(1) as u64, Ordering::Relaxed);
                return Some(entry.clone());
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Inserts or updates the analysis entry for a file, enforcing the maximum capacity limit
    pub fn put(&self, file_path: &str, entry: FileAnalysisEntry) {
        let mut lock = self.entries.write();

        // Eviction policy: if capacity is exceeded, evict the oldest accessed entry
        if lock.len() >= self.max_capacity && !lock.contains_key(file_path) {
            if let Some(oldest_key) = lock
                .iter()
                .min_by_key(|(_, v)| v.last_accessed)
                .map(|(k, _)| k.clone())
            {
                lock.remove(&oldest_key);
            }
        }

        lock.insert(file_path.to_string(), entry);
    }

    /// Invalidates / removes a file from the cache
    pub fn invalidate(&self, file_path: &str) {
        self.entries.write().remove(file_path);
    }

    /// Clears all entries and resets metrics
    pub fn clear(&self) {
        self.entries.write().clear();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        self.saved_embeddings.store(0, Ordering::Relaxed);
    }

    /// Returns a snapshot of cache metrics
    pub fn stats(&self) -> CacheStats {
        let lock = self.entries.read();
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total_requests = hits + misses;
        let hit_rate_percent = if total_requests > 0 {
            (hits as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let total_cached_symbols: usize = lock.values().map(|e| e.symbols.len()).sum();

        CacheStats {
            hits,
            misses,
            cached_files_count: lock.len(),
            total_cached_symbols,
            saved_embeddings_count: self.saved_embeddings.load(Ordering::Relaxed) as usize,
            max_capacity: self.max_capacity,
            hit_rate_percent,
        }
    }
}

impl Default for AstContextCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hashing_standard_vectors() {
        // Standard NIST test vectors
        assert_eq!(
            AstContextCache::compute_sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            AstContextCache::compute_sha256(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            AstContextCache::compute_sha256(b"hello world"),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_cache_hit_and_modification_miss() {
        let cache = AstContextCache::new();
        let file_path = "src/router.rs";
        let initial_code = "pub fn route() -> &'static str { \"local\" }";

        // Initial check -> Miss
        assert!(cache.get_valid_entry(file_path, initial_code).is_none());

        // Put initial entry
        let hash = AstContextCache::compute_content_hash(initial_code);
        let entry = FileAnalysisEntry::new(file_path, hash, vec![], 12, initial_code.len());
        cache.put(file_path, entry);

        // Second check with unchanged code -> Hit
        let hit_entry = cache.get_valid_entry(file_path, initial_code);
        assert!(hit_entry.is_some());

        // Check with modified code -> Miss
        let modified_code = "pub fn route() -> &'static str { \"cloud\" }";
        assert!(cache.get_valid_entry(file_path, modified_code).is_none());

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 2);
    }

    #[test]
    fn test_cache_capacity_eviction() {
        let cache = AstContextCache::with_capacity(2);

        let code = "fn test() {}";
        let hash = AstContextCache::compute_content_hash(code);

        cache.put("file1.rs", FileAnalysisEntry::new("file1.rs", hash.clone(), vec![], 5, 12));
        cache.put("file2.rs", FileAnalysisEntry::new("file2.rs", hash.clone(), vec![], 5, 12));

        assert_eq!(cache.stats().cached_files_count, 2);

        // Access file1 to make it newer
        let _ = cache.get_valid_entry("file1.rs", code);

        // Add file3, should evict file2
        cache.put("file3.rs", FileAnalysisEntry::new("file3.rs", hash.clone(), vec![], 5, 12));

        assert_eq!(cache.stats().cached_files_count, 2);
        assert!(cache.get_valid_entry("file1.rs", code).is_some());
        assert!(cache.get_valid_entry("file3.rs", code).is_some());
    }
}
