use crate::state::AppState;
use locus_context::types::{AssembledContext, ErrorLog, Template};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Deserialize)]
pub struct AssembleRequest {
    pub user_request: String,
    pub templates: Vec<Template>,
    pub errors: Option<Vec<ErrorLog>>,
}

#[derive(Serialize)]
pub struct AssembleResponse {
    pub full_prompt: String,
    pub token_estimate: usize,
    pub sections: PromptSections,
}

#[derive(Serialize)]
pub struct PromptSections {
    pub system_prompt: String,
    pub user_prompt: String,
    pub template_context: String,
    pub error_context: String,
}

#[tauri::command]
pub async fn context_assemble(
    state: State<'_, AppState>,
    request: AssembleRequest,
) -> Result<AssembleResponse, String> {
    let assembler = &state.context_assembler;

    let errors = request.errors.unwrap_or_default();
    let detailed = assembler.assemble_detailed(&request.user_request, request.templates, errors);

    Ok(AssembleResponse {
        full_prompt: detailed.full_prompt,
        token_estimate: detailed.token_estimate,
        sections: PromptSections {
            system_prompt: detailed.system_prompt,
            user_prompt: detailed.user_prompt,
            template_context: detailed.template_context,
            error_context: detailed.error_context,
        },
    })
}

#[tauri::command]
pub async fn context_estimate_tokens(
    state: State<'_, AppState>,
    text: String,
) -> Result<usize, String> {
    let assembler = &state.context_assembler;
    Ok(assembler.estimate_tokens(&text))
}

#[derive(Serialize)]
pub struct TokenFit {
    pub fits: bool,
    pub estimated_tokens: usize,
    pub max_tokens: usize,
}

#[tauri::command]
pub async fn context_fits(
    state: State<'_, AppState>,
    text: String,
) -> Result<TokenFit, String> {
    let assembler = &state.context_assembler;
    let estimated_tokens = assembler.estimate_tokens(&text);
    let max_tokens = 32000;

    Ok(TokenFit {
        fits: estimated_tokens <= max_tokens,
        estimated_tokens,
        max_tokens,
    })
}

#[tauri::command]
pub async fn context_truncate(
    state: State<'_, AppState>,
    text: String,
    max_tokens: usize,
) -> Result<String, String> {
    let assembler = &state.context_assembler;
    let truncated = assembler
        .truncate_to_fit_max(&text, max_tokens);
    Ok(truncated)
}

#[tauri::command]
pub async fn context_semantic_search(
    state: State<'_, AppState>,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<locus_context::vector_types::SemanticSearchResult>, String> {
    let indexer = &state.semantic_indexer;
    indexer
        .search(&query, top_k.unwrap_or(8))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn context_index_file(
    state: State<'_, AppState>,
    file_path: String,
    content: String,
) -> Result<usize, String> {
    let indexer = &state.semantic_indexer;
    indexer
        .index_file(&file_path, &content)
        .await
        .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct ExtractSkeletonResponse {
    pub skeleton: String,
    pub stats: locus_context::SkeletonStats,
}

#[tauri::command]
pub async fn context_extract_skeleton(
    code: String,
    extension: String,
) -> Result<ExtractSkeletonResponse, String> {
    let skeleton = locus_context::extract_skeleton(&code, &extension);
    let stats = locus_context::calculate_skeleton_savings(&code, &skeleton);

    Ok(ExtractSkeletonResponse {
        skeleton,
        stats,
    })
}

// ---------------------------------------------------------------------------
// Pure In-Memory Hybrid Context (SymbolGraph + BM25) Commands
// ---------------------------------------------------------------------------

use locus_context::{Bm25SearchResult, HybridContextEngine, HybridContextPayload, SymbolNode};
use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};

static HYBRID_CONTEXT: Lazy<HybridContextEngine> = Lazy::new(HybridContextEngine::new);

#[tauri::command]
pub async fn context_query_symbol_graph(
    symbol: String,
    path: Option<String>,
) -> Result<Vec<SymbolNode>, String> {
    if let Some(p) = path {
        Ok(HYBRID_CONTEXT.symbol_graph.read().resolve_symbol_context(&symbol, Path::new(&p)))
    } else {
        Ok(HYBRID_CONTEXT.query_symbol(&symbol))
    }
}

#[tauri::command]
pub async fn context_bm25_search(
    query: String,
    limit: Option<usize>,
) -> Result<Vec<Bm25SearchResult>, String> {
    Ok(HYBRID_CONTEXT.bm25_search(&query, limit.unwrap_or(10)))
}

#[tauri::command]
pub async fn context_build_hybrid(
    prompt: String,
    files: Option<Vec<String>>,
    max_tokens: Option<usize>,
) -> Result<HybridContextPayload, String> {
    let target_paths: Vec<PathBuf> = files
        .unwrap_or_default()
        .into_iter()
        .map(PathBuf::from)
        .collect();

    Ok(HYBRID_CONTEXT.build_hybrid_context(&prompt, &target_paths, max_tokens.unwrap_or(4000)))
}


