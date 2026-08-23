//! AstContextCache — In-memory LRU cache keyed by pure FIPS 180-4 SHA-256 digests.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Cached entry holding verified skeleton or extracted metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedEntry {
    pub content_hash: String,
    pub skeleton: String,
    pub symbol_count: usize,
    pub created_at_ms: u64,
    pub last_accessed_ms: u64,
}

/// Thread-safe in-memory cache using FIPS 180-4 SHA-256 digests.
pub struct AstContextCache {
    entries: RwLock<HashMap<[u8; 32], CachedEntry>>,
    max_entries: usize,
    access_counter: AtomicU64,
}

impl Default for AstContextCache {
    fn default() -> Self {
        Self::new(1024)
    }
}

impl AstContextCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::with_capacity(max_entries.min(256))),
            max_entries,
            access_counter: AtomicU64::new(1),
        }
    }

    /// Pure Rust FIPS 180-4 Standard SHA-256 implementation (zero external crypto dependencies).
    pub fn sha256_digest(input: &[u8]) -> [u8; 32] {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
            0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
            0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
            0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
            0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
            0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
            0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
            0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
            0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
            0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
            0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
            0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
            0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
            0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
        ];

        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
            0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
        ];

        let bit_len = (input.len() as u64) * 8;
        let mut msg = input.to_vec();
        msg.push(0x80);
        while (msg.len() % 64) != 56 {
            msg.push(0x00);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());

        for chunk in msg.chunks_exact(64) {
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
                w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
            }

            let mut a = h[0];
            let mut b = h[1];
            let mut c = h[2];
            let mut d = h[3];
            let mut e = h[4];
            let mut f = h[5];
            let mut g = h[6];
            let mut h_val = h[7];

            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let temp1 = h_val.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(maj);

                h_val = g;
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
            h[7] = h[7].wrapping_add(h_val);
        }

        let mut out = [0u8; 32];
        for (i, val) in h.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&val.to_be_bytes());
        }
        out
    }

    /// Converts digest to hex string.
    pub fn hex_digest(digest: &[u8; 32]) -> String {
        let mut s = String::with_capacity(64);
        for b in digest {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    /// Fetches a cached entry by content.
    pub fn get(&self, content: &str) -> Option<CachedEntry> {
        let digest = Self::sha256_digest(content.as_bytes());
        let mut map = self.entries.write();
        if let Some(entry) = map.get_mut(&digest) {
            entry.last_accessed_ms = self.access_counter.fetch_add(1, Ordering::Relaxed);
            Some(entry.clone())
        } else {
            None
        }
    }

    /// Inserts a cached entry, evicting oldest if limit reached.
    pub fn insert(&self, content: &str, skeleton: String, symbol_count: usize) -> String {
        let digest = Self::sha256_digest(content.as_bytes());
        let hex = Self::hex_digest(&digest);
        let seq = self.access_counter.fetch_add(1, Ordering::Relaxed);

        let mut map = self.entries.write();
        if map.len() >= self.max_entries {
            // Evict LRU based on monotonic access sequence
            if let Some((&oldest_key, _)) = map.iter().min_by_key(|(_, v)| v.last_accessed_ms) {
                map.remove(&oldest_key);
            }
        }

        map.insert(digest, CachedEntry {
            content_hash: hex.clone(),
            skeleton,
            symbol_count,
            created_at_ms: seq,
            last_accessed_ms: seq,
        });

        hex
    }

    /// Returns cache size.
    pub fn len(&self) -> usize {
        self.entries.read().len()
    }

    /// Checks if cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.read().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_nist_vector() {
        // Standard NIST test vector: "abc"
        // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad
        let digest = AstContextCache::sha256_digest(b"abc");
        let hex = AstContextCache::hex_digest(&digest);
        assert_eq!(hex, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn test_cache_insert_and_get() {
        let cache = AstContextCache::new(5);
        let code = "pub fn add(a: i32, b: i32) -> i32 { a + b }";
        let skeleton = "pub fn add(a: i32, b: i32) -> i32;";

        let hash = cache.insert(code, skeleton.to_string(), 1);
        assert_eq!(hash.len(), 64);
        assert_eq!(cache.len(), 1);

        let hit = cache.get(code).expect("Cache hit expected");
        assert_eq!(hit.skeleton, skeleton);
        assert_eq!(hit.symbol_count, 1);
    }

    #[test]
    fn test_cache_lru_eviction() {
        let cache = AstContextCache::new(2);
        cache.insert("fn a() {}", "fn a();".into(), 1);
        cache.insert("fn b() {}", "fn b();".into(), 1);
        assert_eq!(cache.len(), 2);

        // Access 'a' to refresh its timestamp
        let _ = cache.get("fn a() {}");

        // Insert 'c' which should evict 'b'
        cache.insert("fn c() {}", "fn c();".into(), 1);
        assert_eq!(cache.len(), 2);
        assert!(cache.get("fn a() {}").is_some());
        assert!(cache.get("fn c() {}").is_some());
        assert!(cache.get("fn b() {}").is_none());
    }
}
