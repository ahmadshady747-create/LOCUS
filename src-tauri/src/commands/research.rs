//! Tauri IPC commands for semantic documentation lookups and compiler error resolution.

use locus_research::{DocQuery, DocSearchEngine, DocSearchResult, Ecosystem, ResolvedErrorSolution};
use once_cell::sync::Lazy;

static RESEARCH_ENGINE: Lazy<DocSearchEngine> = Lazy::new(DocSearchEngine::new);

#[tauri::command]
pub async fn research_fetch_docs(
    query: String,
    ecosystem: String,
    version: Option<String>,
) -> Result<DocSearchResult, String> {
    let eco = Ecosystem::from_str_lenient(&ecosystem);
    let doc_query = DocQuery {
        query,
        ecosystem: eco,
        version,
    };

    RESEARCH_ENGINE
        .search_docs(doc_query)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn research_resolve_error(error_snippet: String) -> Result<ResolvedErrorSolution, String> {
    Ok(RESEARCH_ENGINE.resolve_error(&error_snippet))
}

#[tauri::command]
pub fn research_clear_docs_cache() -> Result<u32, String> {
    RESEARCH_ENGINE.clear_cache().map_err(|e| e.to_string())
}
