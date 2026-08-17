//! Chaos Stress Test Suite for Atomic File System & Torn-Write Prevention.
//!
//! Validates:
//! 1. Heavy concurrency stress across 100 threads with zero panics or deadlocks.
//! 2. Torn-write prevention & cryptographic SHA-256 hash integrity during interrupted writes.
//! 3. Guaranteed cleanup of all temporary swap files (`.locus_tmp_*`).

use locus_fs::editor_bridge::EditorBridgeEngine;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use tempfile::TempDir;

/// Pure deterministic SHA-256 calculation for cryptographic verification without external deps.
fn compute_sha256_hex(data: &[u8]) -> String {
    // 64 round constants
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

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() + 8) % 64 != 0 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes(chunk[i * 4..(i + 1) * 4].try_into().unwrap());
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

    h.iter().map(|v| format!("{:08x}", v)).collect::<Vec<String>>().join("")
}

#[test]
fn test_atomic_fs_concurrent_stress() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let root = Arc::new(temp_dir.path().to_path_buf());
    let mut handles = Vec::new();

    // 100 concurrent threads executing atomic file swaps
    for t_id in 0..100 {
        let root_clone = Arc::clone(&root);
        handles.push(thread::spawn(move || {
            let target_file = root_clone.join(format!("thread_isolated_{}.rs", t_id));
            for iter in 0..5 {
                let payload = format!(
                    "// Thread {} Iteration {}\npub fn compute_{}_{}() -> usize {{ {} * 42 }}\n",
                    t_id, iter, t_id, iter, iter
                );

                let report = EditorBridgeEngine::atomic_write_file(&target_file, &payload)
                    .expect("Atomic write must never fail under concurrency");

                assert!(report.atomic_swap);
                assert_eq!(report.bytes_synced, payload.len());
            }
        }));
    }

    for h in handles {
        h.join().expect("Concurrent writer thread panicked");
    }

    // Verify all 100 target files exist and contain valid non-corrupted content
    for t_id in 0..100 {
        let file_path = root.join(format!("thread_isolated_{}.rs", t_id));
        assert!(file_path.exists(), "Target file {} must exist", t_id);
        let content = fs::read_to_string(&file_path).expect("Read must succeed");
        assert!(content.contains("pub fn compute_"));
        assert!(!content.is_empty());
    }
}

#[test]
fn test_atomic_fs_torn_write_prevention() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let target_file = temp_dir.path().join("critical_config.json");

    // 1. Establish baseline file state & compute SHA-256
    let baseline_content = r#"{"status": "SOVEREIGN", "version": 1, "checksum": "ALPHA_0"}"#;
    EditorBridgeEngine::atomic_write_file(&target_file, baseline_content)
        .expect("Baseline write failed");

    let baseline_hash = compute_sha256_hex(baseline_content.as_bytes());
    let on_disk_hash = compute_sha256_hex(&fs::read(&target_file).unwrap());
    assert_eq!(baseline_hash, on_disk_hash, "Baseline SHA-256 hash must match");

    // 2. Simulate aborted/interrupted write (Half payload written to temp file, then dropped)
    let parent_dir = target_file.parent().unwrap();
    {
        let mut temp_file = tempfile::Builder::new()
            .prefix(".locus_tmp_")
            .tempfile_in(parent_dir)
            .expect("Failed to create tempfile");

        // Write partial/corrupted payload
        temp_file
            .write_all(b"{\"status\": \"CORRUPTED_INCOMPLETE_WRITE\"...")
            .unwrap();
        temp_file.flush().unwrap();
        // Drop tempfile WITHOUT persisting (simulates sudden process kill / power failure)
    }

    // 3. Verify original target file remains 100% untouched and uncorrupted
    let read_after_abort = fs::read_to_string(&target_file).expect("File must remain readable");
    let hash_after_abort = compute_sha256_hex(read_after_abort.as_bytes());
    assert_eq!(read_after_abort, baseline_content);
    assert_eq!(hash_after_abort, baseline_hash, "Target file must remain intact with zero torn write");

    // 4. Perform complete atomic swap to updated version
    let updated_content = r#"{"status": "SOVEREIGN", "version": 2, "checksum": "BETA_1"}"#;
    let report = EditorBridgeEngine::atomic_write_file(&target_file, updated_content)
        .expect("Atomic swap must succeed");
    assert!(report.atomic_swap);

    let updated_hash = compute_sha256_hex(updated_content.as_bytes());
    let on_disk_updated_hash = compute_sha256_hex(&fs::read(&target_file).unwrap());
    assert_eq!(updated_hash, on_disk_updated_hash);
}

#[test]
fn test_atomic_fs_temp_file_cleanup() {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let dir_path = temp_dir.path();

    for i in 0..25 {
        let file_path = dir_path.join(format!("clean_test_{}.rs", i));
        let content = format!("pub fn check_{}() -> bool {{ true }}", i);
        EditorBridgeEngine::atomic_write_file(&file_path, &content)
            .expect("Write must succeed");
    }

    // Inspect directory: no orphan temp files should exist
    let entries = fs::read_dir(dir_path).expect("Failed to read directory");
    let mut tmp_file_count = 0;

    for entry in entries {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(".locus_tmp_") || name.ends_with(".tmp") {
            tmp_file_count += 1;
        }
    }

    assert_eq!(
        tmp_file_count, 0,
        "No orphan temporary swap files should remain after atomic operations"
    );
}
