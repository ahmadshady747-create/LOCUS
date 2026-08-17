//! Tauri IPC commands for Fill-In-the-Middle (FIM) Inline Code Completions.

use locus_context::{FimCompletionRequest, FimCompletionResponse};
use locus_llm::FimDispatcher;
use once_cell::sync::Lazy;

static GLOBAL_FIM_DISPATCHER: Lazy<FimDispatcher> = Lazy::new(FimDispatcher::new);

#[tauri::command]
pub async fn fim_request_inline_completion(
    req: FimCompletionRequest,
) -> Result<FimCompletionResponse, String> {
    GLOBAL_FIM_DISPATCHER.complete(req).await
}
