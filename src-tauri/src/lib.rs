pub mod commands;
pub mod state;

use commands::{
    agents::{
        agent_execute_task, agent_kill, agent_list_active, agent_monitor, agent_spawn,
        agent_status, get_agent_status,
    },
    context::{
        context_assemble, context_estimate_tokens, context_fits, context_truncate,
        context_semantic_search, context_index_file, context_extract_skeleton,
        context_query_symbol_graph, context_bm25_search, context_build_hybrid,
    },
    fs::{
        fs_get_index, fs_modify_file, fs_read_file, fs_scan, fs_search, fs_watch_start,
        fs_write_file, fs_stage_change, fs_accept_change, fs_reject_change, fs_list_staged_changes,
        fs_compute_hunks, fs_accept_hunk, fs_reject_hunk, fs_rollback_last, fs_list_snapshots,
        fs_apply_search_replace,
    },
    llm::{
        llm_chat, llm_detect_models, llm_generate, llm_hybrid_chat, llm_select_best_model,
        llm_set_default_model, send_message, switch_model, toggle_hybrid_mode,
        llm_save_api_key, save_api_key, llm_get_api_key_status, get_configured_providers,
        llm_delete_api_key, delete_api_key, llm_test_api_key, test_provider_connection,
        llm_get_fallback_chain, llm_set_fallback_chain, llm_set_fallback_strategy,
        llm_auto_detect_keys, auto_detect_api_keys, llm_get_key_pool, llm_save_key_pool,
        cognitive_router_route, cognitive_router_classify, cognitive_router_get_strategy,
        cognitive_router_set_strategy, local_discovery_probe_hardware,
        local_discovery_scan_endpoints, local_discovery_get_report,
        model_puller_start_pull, model_puller_get_progress, model_puller_cancel_pull,
        free_provider_radar_get_suggestions, free_provider_radar_dismiss,
        free_provider_radar_save_and_activate,
    },
    network::{
        get_local_devices, network_assign_task, network_discover_devices,
        network_get_local_device, network_load_balancer_state, network_start, network_stop,
    },
    templates::{templates_categories, templates_get, templates_list, templates_reload, templates_search},
    diagnostics::{
        system_get_diagnostics, system_export_diagnostics, diagnostics_run_probe,
        diagnostics_get_active_feed,
    },
    skills::{skills_create, skills_execute, skills_list, skills_rescan, skills_toggle},
    research::{research_fetch_docs, research_resolve_error, research_clear_docs_cache},
    editor_bridge::{
        editor_bridge_status, editor_bridge_sync_file, editor_bridge_open_in_editor,
        editor_bridge_detect_editors,
    },
    security::{security_scan_snippet, security_scan_diff},
    ambient::{ambient_get_snapshot, ambient_paste_to_active},
    slots::{slots_get_config, slots_set_driver, slots_list_available},
    plugins_tools::{
        plugins_list_local_tools, plugins_run_local_tool, plugins_get_circuit_status,
        plugins_reset_circuit,
    },
    plugins_registry::{
        plugins_registry_list, plugins_registry_install_git, plugins_registry_toggle,
        plugins_registry_uninstall,
    },
    airgap::{
        airgap_generate_sync_frames, airgap_ingest_frame, airgap_apply_synced_payload,
        airgap_reset_receiver,
    },
    ergonomics::{
        fs_parse_diff_hunks, fs_apply_selected_hunks, terminal_process_failure,
        context_query_mentions,
    },
    fim::fim_request_inline_completion,
    i18n::{i18n_get_locale, i18n_set_locale},
    verifier::{verifier_prove_contract, verifier_get_active_invariants},
    overlay::{
        toggle_spotlight, get_ambient_telemetry, ambient_controller_dismiss,
        parse_omnibar_input, query_omni_search, search_chat_memory,
        inject_text_to_active, execute_ambient_agent, run_quick_formal_verify,
        get_global_ambient_controller,
    },
    task_graph::{
        task_graph_decompose, task_graph_execute_node, task_graph_update_node,
        task_graph_validate, spotlight_toggle, spotlight_hide, spotlight_show,
        spotlight_set_pinned, spec_aligner_analyze, spec_aligner_apply_tradeoffs,
        adversarial_qa_evaluate, adr_ledger_get, adr_ledger_add_negative,
        adr_ledger_add_record, github_request_device_code, github_poll_token,
        github_get_status, github_logout, github_list_repos, git_get_status,
        git_clone_repo, git_smart_commit, git_create_pull_request,
    },
};
use state::AppState;
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("spotlight") {
                            if let Ok(is_vis) = window.is_visible() {
                                let controller = get_global_ambient_controller();
                                if is_vis {
                                    let _ = window.hide();
                                    controller.dismiss();
                                } else {
                                    let _ = controller.trigger_wake();
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                    let _ = window.set_always_on_top(true);
                                }
                            }
                        }
                    }
                })
                .build(),
        )
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Prevent app termination on window close; minimize to tray instead
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(|app| {
            // Register primary "Alt+Space" and fallback "Ctrl+Shift+L" for OS conflict resistance
            let _ = app.global_shortcut().register("Alt+Space");
            let _ = app.global_shortcut().register("Ctrl+Shift+L");

            // Native System Tray Menu
            let toggle_hud_i = MenuItem::with_id(app, "toggle_hud", "Show / Hide HUD (Alt+Space)", true, None::<&str>)?;
            let verify_i = MenuItem::with_id(app, "verify_check", "Verify Invariants Check", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "open_settings", "Settings", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit LOCUS", true, None::<&str>)?;

            let tray_menu = Menu::with_items(app, &[&toggle_hud_i, &verify_i, &settings_i, &sep, &quit_i])?;

            let mut tray_builder = TrayIconBuilder::with_id("locus-tray")
                .menu(&tray_menu)
                .tooltip("LOCUS — Sovereign Ambient HUD")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
                        "toggle_hud" => {
                            if let Some(window) = app.get_webview_window("spotlight") {
                                if let Ok(is_vis) = window.is_visible() {
                                    let controller = get_global_ambient_controller();
                                    if is_vis {
                                        let _ = window.hide();
                                        controller.dismiss();
                                    } else {
                                        let _ = controller.trigger_wake();
                                        let _ = window.show();
                                        let _ = window.set_focus();
                                        let _ = window.set_always_on_top(true);
                                    }
                                }
                            }
                        }
                        "verify_check" | "open_settings" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.unminimize();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("spotlight") {
                            if let Ok(is_vis) = window.is_visible() {
                                let controller = get_global_ambient_controller();
                                if is_vis {
                                    let _ = window.hide();
                                    controller.dismiss();
                                } else {
                                    let _ = controller.trigger_wake();
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                    let _ = window.set_always_on_top(true);
                                }
                            }
                        }
                    }
                });

            if let Some(icon) = app.default_window_icon() {
                tray_builder = tray_builder.icon(icon.clone());
            }

            let _ = tray_builder.build(app)?;

            let state = tauri::async_runtime::block_on(AppState::new(None));
            app.manage(state);
            Ok(())
        })
.invoke_handler(tauri::generate_handler![
        // File system
        fs_scan,
        fs_read_file,
        fs_write_file,
        fs_modify_file,
        fs_search,
        fs_watch_start,
        fs_get_index,
        fs_stage_change,
        fs_accept_change,
        fs_reject_change,
        fs_list_staged_changes,
        fs_compute_hunks,
        fs_accept_hunk,
        fs_reject_hunk,
        fs_rollback_last,
        fs_list_snapshots,
        // Templates
        templates_list,
        templates_search,
        templates_get,
        templates_categories,
        templates_reload,
        // Context
        context_assemble,
        context_estimate_tokens,
        context_fits,
        context_truncate,
        context_semantic_search,
        context_index_file,
        context_extract_skeleton,
        context_query_symbol_graph,
        context_bm25_search,
        context_build_hybrid,
        // Network
        network_start,
        network_stop,
        network_discover_devices,
        network_get_local_device,
        network_assign_task,
        network_load_balancer_state,
        get_local_devices,
        // Agents
        agent_spawn,
        agent_kill,
        agent_status,
        agent_list_active,
        agent_monitor,
        agent_execute_task,
        get_agent_status,
        // LLM
        llm_detect_models,
        llm_generate,
        llm_chat,
        llm_select_best_model,
        llm_set_default_model,
        llm_hybrid_chat,
        send_message,
        switch_model,
        toggle_hybrid_mode,
        llm_save_api_key,
        save_api_key,
        llm_get_api_key_status,
        get_configured_providers,
        llm_delete_api_key,
        delete_api_key,
        llm_test_api_key,
        test_provider_connection,
        llm_get_fallback_chain,
        llm_set_fallback_chain,
        llm_set_fallback_strategy,
        llm_auto_detect_keys,
        auto_detect_api_keys,
        // System Diagnostics
        system_get_diagnostics,
        system_export_diagnostics,
        // Modular Skills Engine
        skills_list,
        skills_rescan,
        skills_toggle,
        skills_execute,
        skills_create,
        // Free-Tier Optimizer & Token Economy
        llm_get_key_pool,
        llm_save_key_pool,
        fs_apply_search_replace,
        context_extract_skeleton,
        // Mission Control & Task Graph DAG
        task_graph_decompose,
        task_graph_validate,
        task_graph_update_node,
        task_graph_execute_node,
        // Spotlight HUD
        spotlight_toggle,
        spotlight_hide,
        spotlight_show,
        spotlight_set_pinned,
        // Spec Alignment, Adversarial QA, ADR Ledger
        spec_aligner_analyze,
        spec_aligner_apply_tradeoffs,
        adversarial_qa_evaluate,
        adr_ledger_get,
        adr_ledger_add_negative,
        adr_ledger_add_record,
        // GitHub Device Flow & Git Sync
        github_request_device_code,
        github_poll_token,
        github_get_status,
        github_logout,
        github_list_repos,
        git_get_status,
        git_clone_repo,
        git_smart_commit,
        git_create_pull_request,
        // Cognitive Router & Cost-to-Power Optimizer
        cognitive_router_route,
        cognitive_router_classify,
        cognitive_router_get_strategy,
        cognitive_router_set_strategy,
        // Local Discovery & Streaming Model Puller
        local_discovery_probe_hardware,
        local_discovery_scan_endpoints,
        local_discovery_get_report,
        model_puller_start_pull,
        model_puller_get_progress,
        model_puller_cancel_pull,
        // Free Provider Radar & Quota Intelligence
        free_provider_radar_get_suggestions,
        free_provider_radar_dismiss,
        free_provider_radar_save_and_activate,
        // Research & Semantic Docs Extractor
        research_fetch_docs,
        research_resolve_error,
        research_clear_docs_cache,
        // Universal Silent Editor Bridge
        editor_bridge_status,
        editor_bridge_sync_file,
        editor_bridge_open_in_editor,
        editor_bridge_detect_editors,
        // Zero-Shot Micro-SAST Security Gate
        security_scan_snippet,
        security_scan_diff,
        // Ambient OS Context & Active Window Hook
        ambient_get_snapshot,
        ambient_paste_to_active,
        // Swappable Core Slots Engine
        slots_get_config,
        slots_set_driver,
        slots_list_available,
        // Zero-Panic Local Tools & Circuit Breaker
        plugins_list_local_tools,
        plugins_run_local_tool,
        plugins_get_circuit_status,
        plugins_reset_circuit,
        // Decentralized Addon Registry & Git Installer
        plugins_registry_list,
        plugins_registry_install_git,
        plugins_registry_toggle,
        plugins_registry_uninstall,
        // Air-Gapped Animated QR Sync Engine
        airgap_generate_sync_frames,
        airgap_ingest_frame,
        airgap_apply_synced_payload,
        airgap_reset_receiver,
        // Sovereign Ergonomics Suite
        fs_parse_diff_hunks,
        fs_apply_selected_hunks,
        terminal_process_failure,
        context_query_mentions,
        // Fill-In-the-Middle (FIM) & Background Compiler Probe
        fim_request_inline_completion,
        diagnostics_run_probe,
        diagnostics_get_active_feed,
        // Sovereign i18n & Native RTL Engine
        i18n_get_locale,
        i18n_set_locale,
        // Directive-Bound Bidirectional Formal Verifier
        verifier_prove_contract,
        verifier_get_active_invariants,
        // Universal Ambient Overlay & OS Daemon Controller
        toggle_spotlight,
        get_ambient_telemetry,
        ambient_controller_dismiss,
        parse_omnibar_input,
        query_omni_search,
        search_chat_memory,
        inject_text_to_active,
        execute_ambient_agent,
        run_quick_formal_verify,
    ])
    .run(tauri::generate_context!())
    .expect("error while running locus application");
}
