import os
import subprocess
import time
import json

BENCH_DIR = "live_benchmark"
os.makedirs(BENCH_DIR, exist_ok=True)

# 1. كتابة الكود الخام المحتوي على خطأ التزامن الحي (Mutex held across await)
raw_file = os.path.join(BENCH_DIR, "raw_mutex_bug.rs")
with open(raw_file, "w", encoding="utf-8") as f:
    f.write('''use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;

pub struct ServerMetrics {
    pub total_requests: u64,
}

pub fn spawn_faulty_worker(metrics: Arc<Mutex<ServerMetrics>>) {
    tokio::spawn(async move {
        let mut guard = metrics.lock().await;
        guard.total_requests += 1;
        tokio::time::sleep(Duration::from_millis(50)).await;
        guard.total_requests += 1;
    });
}
''')

# 2. كتابة الكود المصحح والمحكوم بحراس لوكيوس (I1 Drop Guard)
safe_file = os.path.join(BENCH_DIR, "locus_mutex_safe.rs")
with open(safe_file, "w", encoding="utf-8") as f:
    f.write('''use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::Duration;

pub struct ServerMetrics {
    pub total_requests: u64,
}

pub fn spawn_safe_worker(metrics: Arc<Mutex<ServerMetrics>>) {
    tokio::spawn(async move {
        {
            let mut guard = metrics.lock().await;
            guard.total_requests += 1;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
        {
            let mut guard = metrics.lock().await;
            guard.total_requests += 1;
        }
    });
}
''')

def run_rustc(file_path: str):
    start = time.perf_counter()
    res = subprocess.run(
        ["rustc", "--crate-type=lib", file_path],
        capture_output=True,
        text=True
    )
    latency = (time.perf_counter() - start) * 1000.0
    return {
        "success": res.returncode == 0,
        "exit_code": res.returncode,
        "stdout": res.stdout,
        "stderr": res.stderr,
        "latency_ms": latency
    }

print("=" * 80)
print(" 🚀 تنفيذ التقييم الحي الفعلي عبر مترجم Rust (Live Compiler Benchmark)")
print("=" * 80)

# تجميع الكود الخام
print("\n[1] تجميع الكود الخام (raw_mutex_bug.rs)...")
raw_result = run_rustc(raw_file)
print(f"    - Exit Code: {raw_result['exit_code']}")
print(f"    - الحالة: {'PASSED' if raw_result['success'] else 'FAILED'}")
if not raw_result["success"]:
    print(f"    - الخطأ الحقيقي الصادر من rustc:\n{raw_result['stderr'][:350]}...")

# تجميع الكود المحكوم
print("\n[2] تجميع الكود المصحح بحوكمة لوكيوس (locus_mutex_safe.rs)...")
safe_result = run_rustc(safe_file)
print(f"    - Exit Code: {safe_result['exit_code']}")
print(f"    - الحالة: {'PASSED' if safe_result['success'] else 'FAILED'}")
print(f"    - زمن الترجمة الفعلي: {safe_result['latency_ms']:.2f} ms")

print("\n" + "=" * 80)
print("✅ تم اكتمال الاختبار الفعلي بالمترجم.")
