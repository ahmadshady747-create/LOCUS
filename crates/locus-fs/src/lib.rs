pub mod diff_engine;
pub mod engine;
pub mod snapshot_store;
pub mod search_replace;
pub mod git_sync;
pub mod editor_bridge;
pub mod hunk_patcher;

pub use diff_engine::{apply_single_hunk, compute_hunks, reject_single_hunk};
pub use hunk_patcher::{
    apply_selected_hunks, check_structural_syntax_safety, parse_diff_into_hunks, PatchResult,
};
pub use engine::FileSystemEngine;
pub use snapshot_store::SnapshotStore;
pub use search_replace::{apply_search_replace_blocks, parse_search_replace_blocks, SearchReplaceBlock};
pub use git_sync::{
    CreatePrRequest, GitCloneOptions, GitStatusReport, GitSyncEngine, PullRequestResult,
    SmartCommitResult,
};
pub use editor_bridge::{
    DetectedEditor, EditorBridgeEngine, EditorBridgeStatus, EditorSyncReport,
};