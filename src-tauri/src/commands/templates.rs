use crate::state::AppState;
use locus_templates::template::Template;
use tauri::State;

#[tauri::command]
pub async fn templates_list(
    state: State<'_, AppState>,
    category: Option<String>,
) -> Result<Vec<Template>, String> {
    let store = &state.template_store;

    let templates = match category {
        Some(cat) => store.get_by_category(&cat).await,
        None => store.load_templates().await,
    };

    Ok(templates)
}

#[tauri::command]
pub async fn templates_search(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<Template>, String> {
    let store = &state.template_store;
    Ok(store.search_templates(&query).await)
}

#[tauri::command]
pub async fn templates_get(
    state: State<'_, AppState>,
    category: String,
    name: String,
) -> Result<Option<Template>, String> {
    let store = &state.template_store;
    Ok(store.get_template(&category, &name).await)
}

#[tauri::command]
pub async fn templates_categories(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let store = &state.template_store;
    Ok(store.get_categories().await)
}

#[tauri::command]
pub async fn templates_reload(state: State<'_, AppState>) -> Result<(), String> {
    let store = &state.template_store;
    store.reload().await.map_err(|e| e.to_string())
}
