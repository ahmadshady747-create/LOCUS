use locus_core::types::{FileSnapshot, RollbackResult};
use locus_core::Result;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

const MAX_SNAPSHOTS: usize = 30;

#[derive(Clone)]
pub struct SnapshotStore {
    snapshots: Arc<RwLock<VecDeque<FileSnapshot>>>,
}

impl SnapshotStore {
    pub fn new() -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_SNAPSHOTS))),
        }
    }

    /// Captures a fast atomic snapshot before any file modification or hunk acceptance.
    pub async fn capture_snapshot(
        &self,
        root: &Path,
        file_path: &Path,
        content: &str,
        description: &str,
    ) -> Result<FileSnapshot> {
        let snapshot_id = format!("snap-{}", Uuid::new_v4());
        let snapshot = FileSnapshot {
            snapshot_id: snapshot_id.clone(),
            created_at: chrono::Utc::now(),
            file_path: file_path.to_path_buf(),
            previous_content: content.to_string(),
            description: description.to_string(),
        };

        // 1. Write backup to disk under .locus/snapshots/
        let snapshot_dir = root.join(".locus").join("snapshots");
        if let Err(e) = tokio::fs::create_dir_all(&snapshot_dir).await {
            warn!("Could not create snapshot dir: {}", e);
        } else {
            let snap_file = snapshot_dir.join(format!("{}.bak", snapshot_id));
            let _ = tokio::fs::write(snap_file, content).await;
        }

        // 2. Add to in-memory deque and enforce MAX_SNAPSHOTS limit (30 items max)
        let mut list = self.snapshots.write().await;
        list.push_front(snapshot.clone());

        while list.len() > MAX_SNAPSHOTS {
            if let Some(removed) = list.pop_back() {
                // Prune old backup file from disk
                let old_file = snapshot_dir.join(format!("{}.bak", removed.snapshot_id));
                let _ = tokio::fs::remove_file(old_file).await;
            }
        }

        info!("Captured atomic snapshot {} for {}", snapshot_id, file_path.display());
        Ok(snapshot)
    }

    /// Restores the most recent snapshot immediately to disk (Rollback Last Action).
    pub async fn rollback_last(&self) -> Result<RollbackResult> {
        let mut list = self.snapshots.write().await;
        if let Some(last_snap) = list.pop_front() {
            let bytes_len = last_snap.previous_content.len();
            tokio::fs::write(&last_snap.file_path, &last_snap.previous_content).await?;

            info!(
                "Rolled back file {} to snapshot {} ({} bytes restored)",
                last_snap.file_path.display(),
                last_snap.snapshot_id,
                bytes_len
            );

            Ok(RollbackResult {
                success: true,
                snapshot_id: last_snap.snapshot_id,
                file_path: last_snap.file_path.clone(),
                restored_bytes: bytes_len,
                message: format!(
                    "Successfully rolled back {} to snapshot before '{}'",
                    last_snap.file_path.file_name().unwrap_or_default().to_string_lossy(),
                    last_snap.description
                ),
            })
        } else {
            Err(locus_core::LocusError::NotFound(
                "No previous snapshots available to rollback.".to_string(),
            ))
        }
    }

    /// Returns a list of all active snapshots.
    pub async fn list_snapshots(&self) -> Vec<FileSnapshot> {
        self.snapshots.read().await.iter().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_snapshot_capture_and_rollback() {
        let temp_dir = std::env::temp_dir().join(format!("locus_test_{}", Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&temp_dir).await;

        let file_path = temp_dir.join("test_code.rs");
        let initial_content = "fn initial() {}\n";
        tokio::fs::write(&file_path, initial_content).await.unwrap();

        let store = SnapshotStore::new();

        // 1. Capture snapshot before applying modification
        let snap = store
            .capture_snapshot(&temp_dir, &file_path, initial_content, "Before AST optimization")
            .await
            .unwrap();
        assert!(snap.snapshot_id.starts_with("snap-"));

        // 2. Modify file on disk
        let modified = "fn modified() { println!(\"new\"); }\n";
        tokio::fs::write(&file_path, modified).await.unwrap();
        assert_eq!(tokio::fs::read_to_string(&file_path).await.unwrap(), modified);

        // 3. Rollback last action
        let rollback = store.rollback_last().await.unwrap();
        assert!(rollback.success);
        assert_eq!(rollback.restored_bytes, initial_content.len());

        // 4. Verify file content restored
        let restored = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(restored, initial_content);

        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }

    #[tokio::test]
    async fn test_max_30_snapshots_retention() {
        let temp_dir = std::env::temp_dir().join(format!("locus_test_ret_{}", Uuid::new_v4()));
        let _ = tokio::fs::create_dir_all(&temp_dir).await;
        let file_path = temp_dir.join("app.rs");

        let store = SnapshotStore::new();
        for i in 0..35 {
            let _ = store
                .capture_snapshot(&temp_dir, &file_path, &format!("content {}", i), "iter")
                .await
                .unwrap();
        }

        let snapshots = store.list_snapshots().await;
        assert_eq!(snapshots.len(), 30); // strictly capped at 30

        let _ = tokio::fs::remove_dir_all(temp_dir).await;
    }
}
