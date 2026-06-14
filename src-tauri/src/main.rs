use app_agent::AgentManager;
use app_core::{
    event_type_name, patch_paths_for, AppEvent, ConversationId, ConversationSearchHit,
    DispatchTimingVm, EffectCommand, EffectTimingVm, Engine, EngineOutput, InitPayload,
    ProcessSample, ResourceBudget, ViewModel, ViewModelPatch,
};
use app_preview::PreviewManager;
use app_process::{spawn_process_group, ProcessSupervisor};
use app_storage::{Db, StorageWrite, StorageWriteResult, StorageWriter};
use app_core::ProjectId;
use app_workspace::{DirtySignal, WorkspaceManager, WorkspaceWatcher};
use crossbeam_channel::{unbounded, Receiver, Sender};
use parking_lot::Mutex;
use serde_json::json;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};
use tokio::sync::mpsc;

mod workspace_dirty;

use workspace_dirty::WorkspaceDirtyCoalescer;

struct Runtime {
    engine: Mutex<Engine>,
    db: Db,
    writer: StorageWriter,
    storage_rx: Mutex<Receiver<StorageWriteResult>>,
    agent_manager: Arc<AgentManager>,
    agent_rx: Mutex<mpsc::UnboundedReceiver<(ConversationId, app_core::AcpMessage)>>,
    process: Mutex<ProcessSupervisor>,
    workspace: WorkspaceManager,
    preview: Mutex<PreviewManager>,
    startup_effects: Mutex<Vec<EffectCommand>>,
    workspace_dirty_tx: Sender<DirtySignal>,
    workspace_dirty_coalescer: Mutex<WorkspaceDirtyCoalescer>,
    workspace_watcher: Mutex<Option<WorkspaceWatcher>>,
    watched_project_id: Mutex<Option<ProjectId>>,
}

#[tauri::command]
fn initial_view_model(state: State<'_, Runtime>) -> Result<EngineOutput, String> {
    state
        .engine
        .lock()
        .snapshot_view_model()
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn dispatch_app_event(
    app: AppHandle,
    state: State<'_, Runtime>,
    event: AppEvent,
) -> Result<EngineOutput, String> {
    let total_start = Instant::now();
    let event_label = event_type_name(&event).to_string();

    let (mut output, vm_before, effects, reduce_ms, initial_patch_ms) = {
        let mut engine = state.engine.lock();
        let vm_before = engine.previous_view_model();
        let (mut output, reduce_ms, initial_patch_ms) =
            engine.handle_input_traced(event).map_err(|e| e.to_string())?;
        let effects = std::mem::take(&mut output.effects);
        (output, vm_before, effects, reduce_ms, initial_patch_ms)
    };

    let (effect_timings, drain_io_ms) = if effects.is_empty() {
        (Vec::new(), 0.0)
    } else {
        let mut timings = Vec::with_capacity(effects.len());
        let drain_io_ms = execute_effects(app, effects, Some(&mut timings)).await?;
        (timings, drain_io_ms)
    };

    let finalize_patch_ms = if effect_timings.is_empty() {
        0.0
    } else {
        let mut engine = state.engine.lock();
        let finalize_start = Instant::now();
        output.patches = engine
            .finalize_after_effects(&vm_before)
            .map_err(|e| e.to_string())?;
        finalize_start.elapsed().as_secs_f64() * 1000.0
    };

    let response_prep_start = Instant::now();
    let patch_count = output.patches.len();
    let patch_paths = patch_paths_for(&output.patches);
    let timing = DispatchTimingVm {
        event: event_label,
        reduce_ms,
        initial_patch_ms,
        effects: effect_timings,
        drain_io_ms,
        finalize_patch_ms,
        response_prep_ms: response_prep_start.elapsed().as_secs_f64() * 1000.0,
        patch_count,
        patch_paths,
        server_total_ms: total_start.elapsed().as_secs_f64() * 1000.0,
    };
    {
        let mut engine = state.engine.lock();
        engine.record_dispatch_timing(timing);
        let history = engine.dispatch_timing_history().to_vec();
        output.patches.push(ViewModelPatch::Replace {
            path: "rightPane.dispatchTimings".into(),
            value: serde_json::to_value(history).map_err(|e| e.to_string())?,
        });
    }

    Ok(output)
}

#[tauri::command]
fn app_ready(app: AppHandle) -> Result<(), String> {
    if let Some(splash) = app.get_webview_window("splashscreen") {
        splash.close().map_err(|e| e.to_string())?;
    }
    if let Some(main) = app.get_webview_window("main") {
        main.show().map_err(|e| e.to_string())?;
        main.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn open_external_target(target: String) -> Result<(), String> {
    let status = Command::new("open")
        .arg(&target)
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("open exited with status {status}"))
    }
}

fn app_db_path() -> PathBuf {
    let mut base = dirs::data_dir()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.push("AgentCockpit");
    base.push("cockpit.sqlite3");
    base
}

fn default_project_path() -> Option<String> {
    std::env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

fn pump_event(app: &AppHandle, event: AppEvent) -> Result<(), String> {
    pump_event_inner(app, event, true)
}

fn pump_event_inner(app: &AppHandle, event: AppEvent, emit: bool) -> Result<(), String> {
    let state = app.state::<Runtime>();
    if emit {
        let output = {
            let mut engine = state.engine.lock();
            engine.handle_input(event).map_err(|e| e.to_string())?
        };
        enqueue_persistence_effects(app, &output.effects)?;
        app.emit("engine://patches", &output)
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let effects = {
        let mut engine = state.engine.lock();
        engine.reduce_event(event).map_err(|e| e.to_string())?
    };
    enqueue_persistence_effects(app, &effects)?;
    Ok(())
}

fn enqueue_persistence_effects(app: &AppHandle, effects: &[EffectCommand]) -> Result<(), String> {
    let state = app.state::<Runtime>();
    for effect in effects {
        match effect {
            EffectCommand::WriteProject {
                effect_id,
                project,
            } => {
                state
                    .writer
                    .enqueue(StorageWrite::UpsertProject {
                        effect_id: effect_id.clone(),
                        project: project.clone(),
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::WriteConversation {
                effect_id,
                conversation,
            } => {
                state
                    .writer
                    .enqueue(StorageWrite::UpsertConversation {
                        effect_id: effect_id.clone(),
                        conversation: conversation.clone(),
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::DeleteConversation {
                effect_id,
                conversation_id,
            } => {
                state
                    .writer
                    .enqueue(StorageWrite::DeleteConversation {
                        effect_id: effect_id.clone(),
                        conversation_id: conversation_id.clone(),
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::WriteConversationMessages {
                effect_id,
                conversation_id,
                messages,
            } => {
                state
                    .writer
                    .enqueue(StorageWrite::InsertMessages {
                        effect_id: effect_id.clone(),
                        conversation_id: conversation_id.clone(),
                        messages: messages.clone(),
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::WriteAcpEvent {
                effect_id,
                conversation_id,
                direction,
                method,
                raw_json,
            } => {
                state
                    .writer
                    .enqueue(StorageWrite::AppendAcpEvent {
                        effect_id: effect_id.clone(),
                        conversation_id: conversation_id.clone(),
                        direction: direction.clone(),
                        method: method.clone(),
                        raw_json: raw_json.clone(),
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::WriteSessionBase { effect_id, session } => {
                state
                    .writer
                    .enqueue(StorageWrite::UpsertSessionBase {
                        effect_id: effect_id.clone(),
                        session: session.clone(),
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::WriteConversationWorkspace {
                effect_id,
                conversation_id,
                workspace,
            } => {
                state
                    .writer
                    .enqueue(StorageWrite::UpsertConversationWorkspace {
                        effect_id: effect_id.clone(),
                        conversation_id: conversation_id.clone(),
                        workspace: workspace.clone(),
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::WriteConversationEditedFiles {
                effect_id,
                conversation_id,
                edited_files,
            } => {
                state
                    .writer
                    .enqueue(StorageWrite::UpsertConversationEditedFiles {
                        effect_id: effect_id.clone(),
                        conversation_id: conversation_id.clone(),
                        edited_files: edited_files.clone(),
                    })
                    .map_err(|e| e.to_string())?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn emit_error(app: &AppHandle, title: &str, detail: impl ToString) {
    let _ = app.emit(
        "engine://diagnostic",
        json!({ "level": "error", "title": title, "detail": detail.to_string() }),
    );
}

fn emit_file_review_load_failed(app: &AppHandle, title: &str, detail: impl ToString) {
    let detail = detail.to_string();
    emit_error(app, title, detail.clone());
    let vm_before = {
        let state = app.state::<Runtime>();
        let engine = state.engine.lock();
        engine.previous_view_model()
    };
    let patches = {
        let state = app.state::<Runtime>();
        let mut engine = state.engine.lock();
        let _ = engine.handle_input(AppEvent::SystemFileReviewLoadFailed {
            message: format!("{title}: {detail}"),
        });
        engine
            .finalize_after_effects(&vm_before)
            .unwrap_or_default()
    };
    emit_engine_patches(app, patches);
}

fn emit_engine_patches(app: &AppHandle, patches: Vec<ViewModelPatch>) {
    if patches.is_empty() {
        return;
    }
    let _ = app.emit(
        "engine://patches",
        &EngineOutput {
            patches,
            effects: vec![],
            diagnostics: vec![],
        },
    );
}

async fn run_startup_effects(app: AppHandle, effects: Vec<EffectCommand>) {
    if effects.is_empty() {
        return;
    }
    let vm_before: ViewModel = {
        let state = app.state::<Runtime>();
        let engine = state.engine.lock();
        engine.previous_view_model()
    };
    if execute_effects(app.clone(), effects, None).await.is_err() {
        return;
    }
    let patches = {
        let state = app.state::<Runtime>();
        let mut engine = state.engine.lock();
        engine
            .finalize_after_effects(&vm_before)
            .unwrap_or_default()
    };
    emit_engine_patches(&app, patches);
}

fn drain_io_events(app: &AppHandle) {
    let state = app.state::<Runtime>();
    let mut storage_events = Vec::new();
    {
        let rx = state.storage_rx.lock();
        while let Ok(result) = rx.try_recv() {
            let event = match result {
                StorageWriteResult::Completed { effect_id } => {
                    AppEvent::SystemStorageWriteCompleted { effect_id }
                }
                StorageWriteResult::Failed { effect_id, error } => {
                    AppEvent::SystemStorageWriteFailed { effect_id, error }
                }
            };
            storage_events.push(event);
        }
    }
    for event in storage_events {
        let _ = pump_event(app, event);
    }

    let mut acp_events = Vec::new();
    {
        let mut rx = state.agent_rx.lock();
        while let Ok((conversation_id, message)) = rx.try_recv() {
            acp_events.push(AppEvent::SystemAcpMessageReceived {
                conversation_id,
                message,
            });
        }
    }
    for event in acp_events {
        let _ = pump_event(app, event);
    }
}

fn drain_process_samples(app: &AppHandle) {
    let state = app.state::<Runtime>();
    let mut process = state.process.lock();
    let samples = process
        .throttle_tick()
        .or_else(|_| process.sample_all())
        .unwrap_or_default();
    drop(process);
    for sample in samples {
        let _ = pump_event(app, AppEvent::SystemProcessSampled { sample });
    }
}

fn sync_workspace_watcher(app: &AppHandle) {
    let state = app.state::<Runtime>();
    let selected = {
        let engine = state.engine.lock();
        engine
            .state()
            .selected_project_id
            .clone()
            .and_then(|project_id| {
                engine
                    .state()
                    .projects
                    .get(&project_id)
                    .map(|project| (project_id, PathBuf::from(&project.path)))
            })
    };

    let currently_watched = state.watched_project_id.lock().clone();
    let target_id = selected.as_ref().map(|(id, _)| id.clone());

    if currently_watched == target_id {
        return;
    }

    *state.workspace_watcher.lock() = None;
    *state.watched_project_id.lock() = None;

    let Some((project_id, root)) = selected else {
        return;
    };

    match WorkspaceWatcher::watch(root, project_id.clone(), state.workspace_dirty_tx.clone()) {
        Ok(watcher) => {
            *state.workspace_watcher.lock() = Some(watcher);
            *state.watched_project_id.lock() = Some(project_id);
        }
        Err(error) => emit_error(app, "Workspace watcher failed", error),
    }
}

fn drain_workspace_dirty(app: &AppHandle) {
    sync_workspace_watcher(app);
    let batches = {
        let state = app.state::<Runtime>();
        let mut coalescer = state.workspace_dirty_coalescer.lock();
        coalescer.drain_ready()
    };
    for (project_id, paths) in batches {
        let _ = pump_event(
            app,
            AppEvent::SystemWorkspaceDirty {
                project_id,
                paths,
            },
        );
    }
}

fn drain_background(app: &AppHandle) {
    drain_io_events(app);
    drain_process_samples(app);
    drain_workspace_dirty(app);
}

async fn execute_effects(
    app: AppHandle,
    effects: Vec<EffectCommand>,
    mut timings: Option<&mut Vec<EffectTimingVm>>,
) -> Result<f64, String> {
    for effect in effects {
        let effect_name = app_core::effect_command_name(&effect).to_string();
        let effect_start = timings.as_ref().map(|_| Instant::now());
        match effect {
            EffectCommand::WriteProject {
                effect_id,
                project,
            } => {
                let state = app.state::<Runtime>();
                state
                    .writer
                    .enqueue(StorageWrite::UpsertProject { effect_id, project })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::WriteConversation {
                effect_id,
                conversation,
            } => {
                let state = app.state::<Runtime>();
                state
                    .writer
                    .enqueue(StorageWrite::UpsertConversation {
                        effect_id,
                        conversation,
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::DeleteConversation {
                effect_id,
                conversation_id,
            } => {
                let state = app.state::<Runtime>();
                state
                    .writer
                    .enqueue(StorageWrite::DeleteConversation {
                        effect_id,
                        conversation_id,
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::WriteConversationMessages {
                effect_id,
                conversation_id,
                messages,
            } => {
                let state = app.state::<Runtime>();
                state
                    .writer
                    .enqueue(StorageWrite::InsertMessages {
                        effect_id,
                        conversation_id,
                        messages,
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::LoadConversationMessages {
                conversation_id,
                ..
            } => {
                let state = app.state::<Runtime>();
                match state.db.load_recent_messages(&conversation_id, 200) {
                    Ok(messages) => pump_event_inner(
                        &app,
                        AppEvent::SystemConversationMessagesLoaded {
                            conversation_id,
                            messages,
                        },
                        false,
                    )?,
                    Err(e) => emit_error(&app, "Conversation messages load failed", e),
                }
            }
            EffectCommand::WriteAcpEvent {
                effect_id,
                conversation_id,
                direction,
                method,
                raw_json,
            } => {
                let state = app.state::<Runtime>();
                state
                    .writer
                    .enqueue(StorageWrite::AppendAcpEvent {
                        effect_id,
                        conversation_id,
                        direction,
                        method,
                        raw_json,
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::WriteSessionBase { effect_id, session } => {
                let state = app.state::<Runtime>();
                state
                    .writer
                    .enqueue(StorageWrite::UpsertSessionBase { effect_id, session })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::WriteConversationWorkspace {
                effect_id,
                conversation_id,
                workspace,
            } => {
                let state = app.state::<Runtime>();
                state
                    .writer
                    .enqueue(StorageWrite::UpsertConversationWorkspace {
                        effect_id,
                        conversation_id,
                        workspace,
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::WriteConversationEditedFiles {
                effect_id,
                conversation_id,
                edited_files,
            } => {
                let state = app.state::<Runtime>();
                state
                    .writer
                    .enqueue(StorageWrite::UpsertConversationEditedFiles {
                        effect_id,
                        conversation_id,
                        edited_files,
                    })
                    .map_err(|e| e.to_string())?;
            }
            EffectCommand::LoadDirectory {
                project_id,
                project_path,
                path,
                ..
            } => {
                let state = app.state::<Runtime>();
                match state
                    .workspace
                    .load_directory(Path::new(&project_path), &path)
                {
                    Ok(children) => pump_event_inner(
                        &app,
                        AppEvent::SystemDirectoryLoaded {
                            project_id,
                            path,
                            children,
                        },
                        false,
                    )?,
                    Err(e) => emit_error(&app, "Directory load failed", e),
                }
            }
            EffectCommand::LoadFilePreview {
                project_id,
                project_path,
                path,
                ..
            } => {
                let state = app.state::<Runtime>();
                match state
                    .workspace
                    .load_file_preview(project_id, Path::new(&project_path), &path)
                {
                    Ok(preview) => {
                        pump_event_inner(&app, AppEvent::SystemFileLoaded { preview }, false)?
                    }
                    Err(e) => emit_file_review_load_failed(&app, "File load failed", e),
                }
            }
            EffectCommand::ComputeChangedFiles {
                project_id,
                project_path,
                ..
            } => {
                let state = app.state::<Runtime>();
                match state
                    .workspace
                    .changed_files(project_id.clone(), Path::new(&project_path))
                {
                    Ok(files) => pump_event_inner(
                        &app,
                        AppEvent::SystemChangedFilesComputed { project_id, files },
                        false,
                    )?,
                    Err(e) => emit_error(&app, "Git status failed", e),
                }
            }
            EffectCommand::ComputeDiff {
                project_id,
                project_path,
                path,
                ..
            } => {
                let state = app.state::<Runtime>();
                match state.workspace.compute_diff(
                    project_id,
                    Path::new(&project_path),
                    path.as_deref(),
                ) {
                    Ok(diff) => {
                        pump_event_inner(&app, AppEvent::SystemDiffComputed { diff }, false)?
                    }
                    Err(e) => emit_error(&app, "Git diff failed", e),
                }
            }
            EffectCommand::RefreshGitOverlay {
                project_id,
                project_path,
                base_revision,
                ..
            } => {
                let state = app.state::<Runtime>();
                match state
                    .workspace
                    .refresh_git_overlay(Path::new(&project_path), &base_revision)
                {
                    Ok(overlay) => pump_event_inner(
                        &app,
                        AppEvent::SystemGitOverlayRefreshed {
                            project_id,
                            overlay,
                        },
                        false,
                    )?,
                    Err(e) => emit_error(&app, "Git overlay refresh failed", e),
                }
            }
            EffectCommand::CaptureSessionBaseRevision {
                conversation_id,
                project_id,
                project_path,
                ..
            } => {
                let state = app.state::<Runtime>();
                match state.workspace.capture_session_base(
                    conversation_id,
                    project_id.clone(),
                    Path::new(&project_path),
                ) {
                    Ok(session) => {
                        pump_event_inner(
                            &app,
                            AppEvent::SystemSessionBaseCaptured { session: session.clone() },
                            false,
                        )?;
                        if let Ok(overlay) = state.workspace.refresh_git_overlay(
                            Path::new(&project_path),
                            &session.revision,
                        ) {
                            pump_event_inner(
                                &app,
                                AppEvent::SystemGitOverlayRefreshed {
                                    project_id,
                                    overlay,
                                },
                                false,
                            )?;
                        }
                    }
                    Err(e) => emit_error(&app, "Session base capture failed", e),
                }
            }
            EffectCommand::LoadPrevFile {
                project_id,
                project_path,
                path,
                base_revision,
                old_path,
                ..
            } => {
                let state = app.state::<Runtime>();
                match state.workspace.load_prev_file_preview(
                    project_id,
                    Path::new(&project_path),
                    &base_revision,
                    &path,
                    old_path.as_deref(),
                ) {
                    Ok(preview) => {
                        pump_event_inner(&app, AppEvent::SystemPrevFileLoaded { preview }, false)?
                    }
                    Err(e) => emit_file_review_load_failed(&app, "Previous revision load failed", e),
                }
            }
            EffectCommand::ComputeStructuredDiff {
                project_id,
                project_path,
                path,
                base_revision,
                old_path,
                status,
                ..
            } => {
                let state = app.state::<Runtime>();
                match state.workspace.compute_structured_diff(
                    project_id,
                    Path::new(&project_path),
                    &base_revision,
                    &path,
                    old_path.as_deref(),
                    status,
                ) {
                    Ok(diff) => pump_event_inner(
                        &app,
                        AppEvent::SystemStructuredDiffComputed { path, diff },
                        false,
                    )?,
                    Err(e) => emit_file_review_load_failed(&app, "Structured diff failed", e),
                }
            }
            EffectCommand::PauseProcessGroup {
                conversation_id, ..
            } => {
                let state = app.state::<Runtime>();
                match state.process.lock().pause(&conversation_id) {
                    Ok(sample) => pump_event(&app, AppEvent::SystemProcessSampled { sample })?,
                    Err(e) => emit_error(&app, "Pause failed", e),
                };
            }
            EffectCommand::ResumeProcessGroup {
                conversation_id, ..
            } => {
                let state = app.state::<Runtime>();
                match state.process.lock().resume(&conversation_id) {
                    Ok(sample) => pump_event(&app, AppEvent::SystemProcessSampled { sample })?,
                    Err(e) => emit_error(&app, "Resume failed", e),
                };
            }
            EffectCommand::KillProcessGroup {
                conversation_id, ..
            } => {
                let state = app.state::<Runtime>();
                match state.process.lock().kill_group(&conversation_id) {
                    Ok(sample) => pump_event(&app, AppEvent::SystemProcessSampled { sample })?,
                    Err(e) => emit_error(&app, "Kill failed", e),
                };
            }
            EffectCommand::UpdateCpuBudget {
                conversation_id,
                cpu_percent,
                ..
            } => {
                let state = app.state::<Runtime>();
                state
                    .process
                    .lock()
                    .update_budget(&conversation_id, cpu_percent);
            }
            EffectCommand::OpenPreview {
                project_id, url, ..
            } => {
                let state = app.state::<Runtime>();
                match state.preview.lock().open(project_id, url.clone()) {
                    Ok(status) => {
                        let label = format!("preview-{}", status.preview_id.as_str());
                        let _ = WebviewWindowBuilder::new(
                            &app,
                            label,
                            WebviewUrl::External(url::Url::parse(&url).map_err(|e| e.to_string())?),
                        )
                        .title("Preview")
                        .build();
                        pump_event(&app, AppEvent::SystemPreviewStatusChanged { status })?;
                    }
                    Err(e) => emit_error(&app, "Preview open failed", e),
                };
            }
            EffectCommand::SuspendPreview { preview_id, .. } => {
                let state = app.state::<Runtime>();
                match state.preview.lock().suspend(&preview_id) {
                    Ok(status) => {
                        pump_event(&app, AppEvent::SystemPreviewStatusChanged { status })?
                    }
                    Err(e) => emit_error(&app, "Preview suspend failed", e),
                };
            }
            EffectCommand::DestroyPreview { preview_id, .. } => {
                let state = app.state::<Runtime>();
                match state.preview.lock().close(&preview_id) {
                    Ok(status) => {
                        pump_event(&app, AppEvent::SystemPreviewStatusChanged { status })?
                    }
                    Err(e) => emit_error(&app, "Preview close failed", e),
                };
            }
            EffectCommand::StartDevServer {
                project_id,
                project_path,
                command,
                args,
                ..
            } => match spawn_process_group(&command, &args, Path::new(&project_path), true) {
                Ok(child) => {
                    let url = "http://localhost:5173".to_string();
                    let state = app.state::<Runtime>();
                    match state.preview.lock().open(project_id, url) {
                        Ok(mut status) => {
                            status.dev_server_pid = Some(child.id() as i32);
                            pump_event(&app, AppEvent::SystemPreviewStatusChanged { status })?;
                        }
                        Err(e) => emit_error(&app, "Dev server preview failed", e),
                    };
                }
                Err(e) => emit_error(&app, "Dev server start failed", e),
            },
            EffectCommand::StartCursorAcp {
                conversation_id,
                project_path,
                resume_session_id,
                ..
            } => {
                let app_for_task = app.clone();
                let manager = app.state::<Runtime>().agent_manager.clone();
                tauri::async_runtime::spawn(async move {
                    match manager
                        .start_or_resume_session(
                            conversation_id.clone(),
                            PathBuf::from(&project_path),
                            resume_session_id,
                        )
                        .await
                    {
                        Ok((root_pid, session_id, session_meta, suppress_replay)) => {
                            if let Some(root_pid) = root_pid {
                                let state = app_for_task.state::<Runtime>();
                                let pgid = root_pid;
                                let sample: ProcessSample = state.process.lock().register(
                                    conversation_id.clone(),
                                    root_pid,
                                    pgid,
                                    ResourceBudget::default(),
                                );
                                let _ = pump_event(
                                    &app_for_task,
                                    AppEvent::SystemAcpStarted {
                                        conversation_id: conversation_id.clone(),
                                        root_pid,
                                        pgid,
                                    },
                                );
                                let _ = pump_event(
                                    &app_for_task,
                                    AppEvent::SystemProcessSampled { sample },
                                );
                            }
                            let _ = pump_event(
                                &app_for_task,
                                AppEvent::SystemAcpSessionReady {
                                    conversation_id: conversation_id.clone(),
                                    cursor_session_id: session_id,
                                    suppress_replay,
                                },
                            );
                            let _ = pump_event(
                                &app_for_task,
                                AppEvent::SystemAcpSessionMetaReceived {
                                    conversation_id,
                                    payload: session_meta,
                                },
                            );
                        }
                        Err(e) => {
                            emit_error(&app_for_task, "Cursor ACP start failed", e);
                            let _ = pump_event(
                                &app_for_task,
                                AppEvent::SystemAcpStartFailed { conversation_id },
                            );
                        }
                    }
                });
            }
            EffectCommand::SetAcpMode {
                conversation_id,
                cursor_session_id,
                mode_id,
                ..
            } => {
                let app_for_task = app.clone();
                let manager = app.state::<Runtime>().agent_manager.clone();
                tauri::async_runtime::spawn(async move {
                    match manager
                        .set_mode(&conversation_id, cursor_session_id.as_deref(), &mode_id)
                        .await
                    {
                        Ok(payload) => {
                            let _ = pump_event(
                                &app_for_task,
                                AppEvent::SystemAcpSessionMetaReceived {
                                    conversation_id,
                                    payload,
                                },
                            );
                        }
                        Err(e) => emit_error(&app_for_task, "ACP mode change failed", e),
                    }
                });
            }
            EffectCommand::SetAcpConfigOption {
                conversation_id,
                cursor_session_id,
                config_id,
                value_id,
                ..
            } => {
                let app_for_task = app.clone();
                let manager = app.state::<Runtime>().agent_manager.clone();
                tauri::async_runtime::spawn(async move {
                    match manager
                        .set_config_option(
                            &conversation_id,
                            cursor_session_id.as_deref(),
                            &config_id,
                            &value_id,
                        )
                        .await
                    {
                        Ok(payload) => {
                            let _ = pump_event(
                                &app_for_task,
                                AppEvent::SystemAcpSessionMetaReceived {
                                    conversation_id,
                                    payload,
                                },
                            );
                        }
                        Err(e) => emit_error(&app_for_task, "ACP model change failed", e),
                    }
                });
            }
            EffectCommand::SendAcpPrompt {
                conversation_id,
                cursor_session_id,
                text,
                ..
            } => {
                let app_for_task = app.clone();
                let manager = app.state::<Runtime>().agent_manager.clone();
                tauri::async_runtime::spawn(async move {
                    match manager
                        .send_prompt(&conversation_id, cursor_session_id.as_deref(), &text)
                        .await
                    {
                        Ok(_) => {
                            let _ = pump_event(
                                &app_for_task,
                                AppEvent::SystemAcpPromptCompleted { conversation_id },
                            );
                        }
                        Err(e) => emit_error(&app_for_task, "ACP prompt failed", e),
                    }
                });
            }
            EffectCommand::RespondAcpPermission {
                conversation_id,
                acp_request_id,
                option_id,
                ..
            } => {
                let app_for_task = app.clone();
                let manager = app.state::<Runtime>().agent_manager.clone();
                tauri::async_runtime::spawn(async move {
                    let request_id = acp_request_id.unwrap_or_else(|| json!(null));
                    if let Err(e) = manager
                        .respond_permission(&conversation_id, request_id, &option_id)
                        .await
                    {
                        emit_error(&app_for_task, "ACP permission response failed", e);
                    }
                });
            }
            EffectCommand::CancelAcpSession {
                conversation_id,
                cursor_session_id,
                ..
            } => {
                let app_for_task = app.clone();
                let manager = app.state::<Runtime>().agent_manager.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = manager
                        .cancel(&conversation_id, cursor_session_id.as_deref())
                        .await
                    {
                        emit_error(&app_for_task, "ACP cancel failed", e);
                    }
                });
            }
            EffectCommand::SearchMessages { query, limit, .. } => {
                let state = app.state::<Runtime>();
                let fts_query = query
                    .split_whitespace()
                    .map(|word| format!("\"{word}\""))
                    .collect::<Vec<_>>()
                    .join(" OR ");
                let hits = state
                    .db
                    .search_messages(&fts_query, limit)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|hit| ConversationSearchHit {
                        conversation_id: hit.conversation_id,
                        snippet: hit.snippet,
                    })
                    .collect();
                pump_event(
                    &app,
                    AppEvent::SystemMessageSearchResults { hits },
                )?;
            }
            EffectCommand::BuildFilenameIndex {
                project_id,
                project_path,
                ..
            } => {
                let state = app.state::<Runtime>();
                match state
                    .workspace
                    .build_filename_index(Path::new(&project_path))
                {
                    Ok(entries) => pump_event(
                        &app,
                        AppEvent::SystemFilenameIndexReady {
                            project_id,
                            entries,
                        },
                    )?,
                    Err(e) => emit_error(&app, "Filename index build failed", e),
                }
            }
            EffectCommand::SearchWorkspace {
                project_id,
                project_path,
                query,
                mode,
                ..
            } => {
                let state = app.state::<Runtime>();
                let index = {
                    let engine = state.engine.lock();
                    engine
                        .state()
                        .filename_indexes
                        .get(&project_id)
                        .cloned()
                };
                match state.workspace.search_workspace(
                    Path::new(&project_path),
                    index.as_deref(),
                    &query,
                    &mode,
                    50,
                ) {
                    Ok(hits) => pump_event(
                        &app,
                        AppEvent::SystemSearchResultsPartial {
                            project_id,
                            hits,
                            done: true,
                        },
                    )?,
                    Err(e) => emit_error(&app, "Workspace search failed", e),
                }
            }
            EffectCommand::SteerAcpPrompt {
                conversation_id,
                cursor_session_id,
                text,
                ..
            } => {
                let app_for_task = app.clone();
                let manager = app.state::<Runtime>().agent_manager.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = manager
                        .steer_prompt(&conversation_id, cursor_session_id.as_deref(), &text)
                        .await
                    {
                        emit_error(&app_for_task, "ACP steer failed", e);
                    }
                });
            }
            EffectCommand::StopAcpSession {
                conversation_id,
                cursor_session_id,
                root_pid: _,
                ..
            } => {
                let app_for_task = app.clone();
                let manager = app.state::<Runtime>().agent_manager.clone();
                let conversation_id_for_kill = conversation_id.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = manager
                        .stop_session(&conversation_id, cursor_session_id.as_deref())
                        .await
                    {
                        emit_error(&app_for_task, "ACP stop failed", e);
                    }
                    let state = app_for_task.state::<Runtime>();
                    let _ = state
                        .process
                        .lock()
                        .kill_group(&conversation_id_for_kill);
                });
            }
            EffectCommand::UnregisterProcessGroup { conversation_id, .. } => {
                let state = app.state::<Runtime>();
                state.process.lock().unregister(&conversation_id);
            }
        }
        if let (Some(timings), Some(start)) = (timings.as_mut(), effect_start) {
            timings.push(EffectTimingVm {
                name: effect_name,
                duration_ms: start.elapsed().as_secs_f64() * 1000.0,
            });
        }
    }
    let drain_start = Instant::now();
    drain_io_events(&app);
    let drain_io_ms = drain_start.elapsed().as_secs_f64() * 1000.0;
    Ok(drain_io_ms)
}

fn main() {
    let db = Db::open(app_db_path()).expect("open sqlite store");
    let (storage_tx, storage_rx) = unbounded();
    let writer = StorageWriter::start(db.clone(), storage_tx);
    let (agent_tx, agent_rx) = mpsc::unbounded_channel();
    let agent_manager = Arc::new(AgentManager::new(agent_tx));
    let mut engine = Engine::new(InitPayload {
        initial_project_path: None,
    })
    .expect("create engine");
    let projects = db.load_projects().unwrap_or_default();
    if projects.is_empty() {
        if let Some(path) = default_project_path() {
            let project = app_core::Project::new(path);
            let _ = db.upsert_project(&project);
            engine.hydrate_projects(vec![project]);
        }
    } else {
        engine.hydrate_projects(projects);
    }
    let conversations = db.load_conversations(200).unwrap_or_default();
    let session_bases = db.load_session_bases().unwrap_or_default();
    let conversation_workspaces = db.load_conversation_workspaces().unwrap_or_default();
    let conversation_edited_files = db.load_conversation_edited_files().unwrap_or_default();
    engine.hydrate_persisted_conversation_state(
        session_bases,
        conversation_workspaces,
        conversation_edited_files,
    );
    let (workspace_dirty_tx, workspace_dirty_rx) = unbounded();
    let mut startup_effects = engine.hydrate_conversations(conversations, BTreeMap::new());
    for project in engine.state().projects.values() {
        startup_effects.push(EffectCommand::BuildFilenameIndex {
            effect_id: app_core::EffectId::new(),
            project_id: project.id.clone(),
            project_path: project.path.clone(),
        });
    }
    if let Some(project_id) = engine.state().selected_project_id.clone() {
        if let Some(project) = engine.state().projects.get(&project_id) {
            let base_revision = engine
                .state()
                .selected_conversation_id
                .as_ref()
                .and_then(|id| engine.state().session_base_revisions.get(id))
                .filter(|session| session.project_id == project_id)
                .map(|session| session.revision.clone())
                .unwrap_or_else(|| "HEAD".into());
            startup_effects.push(EffectCommand::LoadDirectory {
                effect_id: app_core::EffectId::new(),
                project_id: project_id.clone(),
                project_path: project.path.clone(),
                path: ".".into(),
            });
            startup_effects.push(EffectCommand::RefreshGitOverlay {
                effect_id: app_core::EffectId::new(),
                project_id,
                project_path: project.path.clone(),
                base_revision,
            });
        }
    }
    let runtime = Runtime {
        engine: Mutex::new(engine),
        db,
        writer,
        storage_rx: Mutex::new(storage_rx),
        agent_manager,
        agent_rx: Mutex::new(agent_rx),
        process: Mutex::new(ProcessSupervisor::new()),
        workspace: WorkspaceManager::new(),
        preview: Mutex::new(PreviewManager::new()),
        startup_effects: Mutex::new(startup_effects),
        workspace_dirty_tx,
        workspace_dirty_coalescer: Mutex::new(WorkspaceDirtyCoalescer::new(workspace_dirty_rx)),
        workspace_watcher: Mutex::new(None),
        watched_project_id: Mutex::new(None),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(runtime)
        .invoke_handler(tauri::generate_handler![
            initial_view_model,
            dispatch_app_event,
            app_ready,
            open_external_target
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let effects = {
                    let state = handle.state::<Runtime>();
                    let mut pending = state.startup_effects.lock();
                    std::mem::take(&mut *pending)
                };
                run_startup_effects(handle.clone(), effects).await;
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut tick = tokio::time::interval(Duration::from_millis(500));
                loop {
                    tick.tick().await;
                    drain_background(&handle);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("run tauri app");
}
