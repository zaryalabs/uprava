//! Workspace boundary enforcement, files, commands, diffs and snapshots.

use super::*;

pub(crate) fn workspace_validation_events(
    config: &NodeConfig,
    command: &CommandEnvelope,
    placement_seqs: &mut HashMap<String, i64>,
) -> Vec<EventEnvelope> {
    placement_snapshot_events(
        config,
        command,
        placement_seqs,
        EventKind::WorkspaceValidated,
    )
}

pub(crate) fn resource_snapshot_events(
    config: &NodeConfig,
    command: &CommandEnvelope,
    placement_seqs: &mut HashMap<String, i64>,
) -> Vec<EventEnvelope> {
    placement_snapshot_events(
        config,
        command,
        placement_seqs,
        EventKind::ResourceSnapshotUpdated,
    )
}

pub(crate) fn placement_snapshot_events(
    config: &NodeConfig,
    command: &CommandEnvelope,
    placement_seqs: &mut HashMap<String, i64>,
    event_kind: EventKind,
) -> Vec<EventEnvelope> {
    let Some(project_placement_id) = command.target.project_placement_id().cloned() else {
        return vec![];
    };
    let workspace_path = command_payload_str(command, "workspace_path").unwrap_or("");
    let display_name = command_payload_str(command, "display_name").unwrap_or("workspace");
    let snapshot = validate_command_workspace(config, display_name, workspace_path);
    let placement_id_payload = project_placement_id.as_str().to_owned();
    vec![placement_event_for_command(
        command,
        placement_seqs,
        project_placement_id,
        event_kind,
        serde_json::json!({
            "placement_id": placement_id_payload,
            "display_name": snapshot.display_name,
            "workspace_path": snapshot.workspace_path,
            "state": snapshot.state,
            "resource_badges": snapshot.resource_badges,
            "git_snapshot": snapshot.git_snapshot,
            "last_validated_at": snapshot.last_validated_at,
        }),
    )]
}

pub(crate) fn validate_command_workspace(
    config: &NodeConfig,
    display_name: &str,
    workspace_path: &str,
) -> WorkspaceSnapshot {
    let path = Path::new(workspace_path);
    if !workspace_path_allowed(config, path) {
        return WorkspaceSnapshot {
            display_name: display_name.to_owned(),
            workspace_path: workspace_path.to_owned(),
            state: PlacementState::Error,
            resource_badges: vec![ResourceBadge {
                kind: "workspace_outside_allowed_roots".to_owned(),
                severity: WarningSeverity::HardBlock,
                label: "Workspace outside allowed roots".to_owned(),
            }],
            git_snapshot: None,
            last_validated_at: Utc::now(),
        };
    }

    let mut snapshot = validate_workspace(path);
    snapshot.display_name = display_name.to_owned();
    snapshot
}

pub(crate) fn workspace_path_allowed(config: &NodeConfig, path: &Path) -> bool {
    if let Ok(canonical_path) = std::fs::canonicalize(path) {
        return canonical_workspace_path_allowed(config, &canonical_path);
    }
    path.ancestors()
        .skip(1)
        .find_map(|ancestor| std::fs::canonicalize(ancestor).ok())
        .is_some_and(|ancestor| canonical_workspace_path_allowed(config, &ancestor))
}

pub(crate) fn workspace_tree_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> (CommandState, JsonValue) {
    match build_workspace_tree_response(config, command) {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

pub(crate) fn workspace_file_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> (CommandState, JsonValue) {
    match build_workspace_file_response(config, command) {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

pub(crate) fn workspace_file_write_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> (CommandState, JsonValue) {
    match write_workspace_file(config, command) {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

pub(crate) async fn workspace_command_run_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> (CommandState, JsonValue) {
    match run_workspace_command(config, command).await {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

pub(crate) async fn workspace_diff_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> (CommandState, JsonValue) {
    match build_workspace_diff_response(config, command).await {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

pub(crate) async fn workspace_terminal_open_command_result(
    config: &NodeConfig,
    command: &CommandEnvelope,
    live_sender: Option<&ControlFrameSender>,
    terminal_supervisor: Option<&TerminalSupervisor>,
) -> (CommandState, JsonValue) {
    let Some(sender) = live_sender else {
        return (
            CommandState::Failed,
            WorkspaceInspectError::new(
                "workspace_terminal.control_unavailable",
                "Workspace terminal requires a live node control channel",
            )
            .into_payload(),
        );
    };
    let Some(terminal_supervisor) = terminal_supervisor else {
        return (
            CommandState::Failed,
            WorkspaceInspectError::new(
                "workspace_terminal.manager_unavailable",
                "Workspace terminal manager is unavailable",
            )
            .into_payload(),
        );
    };
    match terminal_supervisor.open(config, command, sender).await {
        Ok(response) => workspace_success_payload(response),
        Err(error) => (CommandState::Failed, error.into_payload()),
    }
}

pub(crate) fn workspace_success_payload<T: Serialize>(value: T) -> (CommandState, JsonValue) {
    match serde_json::to_value(value) {
        Ok(value) => (CommandState::Completed, JsonValue(value)),
        Err(error) => (
            CommandState::Failed,
            JsonValue(serde_json::json!({
                "error_code": "workspace.serialization_failed",
                "message": error.to_string(),
            })),
        ),
    }
}

pub(crate) fn write_workspace_file(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> Result<WorkspaceFileWriteResponse, WorkspaceInspectError> {
    let request = workspace_command_payload::<WorkspaceFileWriteRequest>(command)?;
    let placement_id = workspace_command_placement_id(command)?;
    let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
    let relative_path = safe_workspace_relative_path(&request.path)?;
    if relative_path.as_os_str().is_empty() {
        return Err(WorkspaceInspectError::new(
            "workspace.path_required",
            "Workspace file write requires a file path",
        ));
    }
    if let Some(status) = generated_or_ignored_status(&relative_path) {
        return Err(WorkspaceInspectError::new(
            "workspace.protected_path",
            format!(
                "Workspace file writes cannot target {} paths",
                workspace_entry_status_label(status)
            ),
        ));
    }
    if request.content.len() > MAX_WORKSPACE_TEXT_BYTES as usize {
        return Err(WorkspaceInspectError::new(
            "workspace.write_too_large",
            format!(
                "Workspace file writes are limited to {} bytes",
                MAX_WORKSPACE_TEXT_BYTES
            ),
        ));
    }
    if request.content.as_bytes().contains(&0) {
        return Err(WorkspaceInspectError::new(
            "workspace.write_binary_content",
            "Workspace file writes only accept text content",
        ));
    }

    let parent_relative = relative_path.parent().unwrap_or_else(|| Path::new(""));
    let file_name = relative_path.file_name().ok_or_else(|| {
        WorkspaceInspectError::new(
            "workspace.path_required",
            "Workspace file write requires a file name",
        )
    })?;
    let parent_path = resolve_existing_workspace_path(&workspace_root, parent_relative)?;
    let parent_metadata = fs::symlink_metadata(&parent_path).map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.parent_metadata_failed",
            format!("Failed to inspect parent directory: {error}"),
        )
    })?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(WorkspaceInspectError::new(
            "workspace.parent_not_directory",
            "Workspace file write parent is not a directory",
        ));
    }
    let target_path = parent_path.join(file_name);
    if binary_extension(&relative_path) {
        return Err(WorkspaceInspectError::new(
            "workspace.write_binary_file",
            "Workspace file writes do not edit binary file types",
        ));
    }

    let mut file = open_workspace_write_target(
        &target_path,
        &relative_path,
        request.expected_content.as_deref(),
    )?;
    file.set_len(0).map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.write_failed",
            format!("Failed to truncate {}: {error}", relative_path.display()),
        )
    })?;
    file.seek(SeekFrom::Start(0)).map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.write_failed",
            format!("Failed to seek {}: {error}", relative_path.display()),
        )
    })?;
    file.write_all(request.content.as_bytes())
        .map_err(|error| {
            WorkspaceInspectError::new(
                "workspace.write_failed",
                format!("Failed to write {}: {error}", relative_path.display()),
            )
        })?;
    file.sync_all().map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.write_failed",
            format!("Failed to sync {}: {error}", relative_path.display()),
        )
    })?;

    let metadata = file.metadata().map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.metadata_failed",
            format!(
                "Failed to inspect written file {}: {error}",
                relative_path.display()
            ),
        )
    })?;
    Ok(WorkspaceFileWriteResponse {
        placement_id,
        path: relative_path_string(&relative_path),
        metadata: WorkspaceEntry {
            name: workspace_entry_name(&relative_path),
            path: relative_path_string(&relative_path),
            kind: workspace_entry_kind(&metadata),
            status: workspace_status_for_entry(&relative_path, &metadata),
            classification: workspace_entry_classification(&relative_path),
            expandable: false,
            byte_len: metadata.is_file().then_some(metadata.len()),
            modified_at: metadata_modified_at(&metadata),
            children: vec![],
        },
        edit_id: format!("workspace-edit-{}", command.command_id.as_str()),
        written_at: Utc::now(),
    })
}

pub(crate) fn open_workspace_write_target(
    target_path: &Path,
    relative_path: &Path,
    expected_content: Option<&str>,
) -> Result<fs::File, WorkspaceInspectError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true);
    set_no_follow(&mut options);
    match options.open(target_path) {
        Ok(mut file) => {
            validate_opened_write_target(&mut file, relative_path, expected_content)?;
            Ok(file)
        }
        Err(error) if error.kind() == ErrorKind::NotFound && expected_content.is_none() => {
            let mut create_options = OpenOptions::new();
            create_options.read(true).write(true).create_new(true);
            set_no_follow(&mut create_options);
            create_options.open(target_path).map_err(|error| {
                if is_symlink_open_error(&error) {
                    WorkspaceInspectError::new(
                        "workspace.write_symlink",
                        "Workspace file writes do not follow symlinks",
                    )
                } else if error.kind() == ErrorKind::AlreadyExists {
                    WorkspaceInspectError::new(
                        "workspace.write_conflict",
                        "Workspace file changed before save; reload before writing",
                    )
                } else {
                    WorkspaceInspectError::new(
                        "workspace.write_failed",
                        format!(
                            "Failed to create {} for writing: {error}",
                            relative_path.display()
                        ),
                    )
                }
            })
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Err(WorkspaceInspectError::new(
            "workspace.write_conflict",
            "Workspace file changed before save: target is now missing",
        )),
        Err(error) if is_symlink_open_error(&error) => Err(WorkspaceInspectError::new(
            "workspace.write_symlink",
            "Workspace file writes do not follow symlinks",
        )),
        Err(error) => Err(WorkspaceInspectError::new(
            "workspace.write_failed",
            format!(
                "Failed to open {} for writing: {error}",
                relative_path.display()
            ),
        )),
    }
}

pub(crate) fn validate_opened_write_target(
    file: &mut fs::File,
    relative_path: &Path,
    expected_content: Option<&str>,
) -> Result<(), WorkspaceInspectError> {
    let metadata = file.metadata().map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.metadata_failed",
            format!("Failed to inspect {}: {error}", relative_path.display()),
        )
    })?;
    if !metadata.is_file() {
        return Err(WorkspaceInspectError::new(
            "workspace.write_not_file",
            "Workspace file write target is not a file",
        ));
    }
    if metadata.len() > MAX_WORKSPACE_TEXT_BYTES {
        return Err(WorkspaceInspectError::new(
            "workspace.write_large_file",
            "Workspace file write target is too large for lightweight editing",
        ));
    }
    file.seek(SeekFrom::Start(0)).map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.read_failed",
            format!(
                "Failed to seek {} before writing: {error}",
                relative_path.display()
            ),
        )
    })?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.read_failed",
            format!(
                "Failed to read {} before writing: {error}",
                relative_path.display()
            ),
        )
    })?;
    if bytes.contains(&0) {
        return Err(WorkspaceInspectError::new(
            "workspace.write_binary_file",
            "Workspace file writes do not edit binary content",
        ));
    }
    let current_content = String::from_utf8(bytes).map_err(|_| {
        WorkspaceInspectError::new(
            "workspace.write_binary_file",
            "Workspace file writes only edit UTF-8 text",
        )
    })?;
    if let Some(expected_content) = expected_content {
        if current_content != expected_content {
            return Err(WorkspaceInspectError::new(
                "workspace.write_conflict",
                "Workspace file changed before save; reload before writing",
            ));
        }
    }
    Ok(())
}

pub(crate) fn set_no_follow(options: &mut OpenOptions) {
    #[cfg(unix)]
    {
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(not(unix))]
    {
        let _ = options;
    }
}

pub(crate) fn is_symlink_open_error(error: &std::io::Error) -> bool {
    #[cfg(unix)]
    {
        error.raw_os_error() == Some(libc::ELOOP)
    }
    #[cfg(not(unix))]
    {
        let _ = error;
        false
    }
}

pub(crate) async fn run_workspace_command(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> Result<WorkspaceCommandRunResponse, WorkspaceInspectError> {
    let request = workspace_command_payload::<WorkspaceCommandRunRequest>(command)?;
    validate_workspace_command_request(&request)?;
    let placement_id = workspace_command_placement_id(command)?;
    let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
    let timeout_seconds = request
        .timeout_seconds
        .unwrap_or(30)
        .clamp(1, MAX_WORKSPACE_COMMAND_SECONDS);
    let output = run_workspace_process(
        &workspace_root,
        request.command.trim(),
        &request.args,
        Duration::from_secs(timeout_seconds),
        MAX_WORKSPACE_COMMAND_OUTPUT_BYTES,
        MAX_WORKSPACE_COMMAND_OUTPUT_BYTES,
    )
    .await;
    Ok(WorkspaceCommandRunResponse {
        placement_id,
        terminal_command_id: format!("terminal-command-{}", command.command_id.as_str()),
        command: request.command.trim().to_owned(),
        args: request.args,
        intent: request.intent,
        label: request.label,
        exit_code: output.exit_code,
        success: output.success,
        stdout: output.stdout,
        stderr: output.stderr,
        stdout_truncated: output.stdout_truncated,
        stderr_truncated: output.stderr_truncated,
        duration_ms: output.duration_ms,
        started_at: output.started_at,
        completed_at: output.completed_at,
    })
}

pub(crate) async fn build_workspace_diff_response(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> Result<WorkspaceDiffResponse, WorkspaceInspectError> {
    let request = workspace_command_payload::<WorkspaceDiffRequest>(command)?;
    let placement_id = workspace_command_placement_id(command)?;
    let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
    let relative_path = request
        .path
        .as_deref()
        .map(safe_workspace_relative_path)
        .transpose()?;
    if relative_path
        .as_ref()
        .is_some_and(|path| path.as_os_str().is_empty())
    {
        return Err(WorkspaceInspectError::new(
            "workspace.path_required",
            "Workspace diff path must identify a file",
        ));
    }
    let inside = run_workspace_process(
        &workspace_root,
        "git",
        &["rev-parse".to_owned(), "--is-inside-work-tree".to_owned()],
        Duration::from_secs(10),
        1_024,
        4_096,
    )
    .await;
    if !inside.success || inside.stdout.trim() != "true" {
        let git_snapshot = GitWorkspaceSnapshot {
            state: GitRepositoryState::NotRepository,
            repo_id: None,
            head_state: None,
            branch: None,
            commit: None,
            upstream: None,
            ahead: 0,
            behind: 0,
            worktree_kind: None,
            operation: None,
            changed_files: vec![],
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
            conflicted_count: 0,
            truncated: false,
            generated_at: Utc::now(),
        };
        return Ok(WorkspaceDiffResponse {
            placement_id,
            diff_id: format!("workspace-diff-{}", command.command_id.as_str()),
            git_snapshot,
            summary: "Workspace is not a git worktree".to_owned(),
            diff: inside.stderr,
            scope: request.scope,
            path: request.path,
            changed_files: vec![],
            hunks: vec![],
            original: None,
            modified: None,
            binary: false,
            summary_truncated: false,
            diff_truncated: inside.stderr_truncated,
            generated_at: Utc::now(),
        });
    }
    let snapshot_root = workspace_root.clone();
    let snapshot = tokio::task::spawn_blocking(move || git_workspace_snapshot(&snapshot_root))
        .await
        .map_err(|error| {
            WorkspaceInspectError::new(
                "workspace.git_snapshot_failed",
                format!("Git snapshot worker failed: {error}"),
            )
        })?
        .ok_or_else(|| {
            WorkspaceInspectError::new(
                "workspace.git_snapshot_failed",
                "Git snapshot could not be collected",
            )
        })?;
    let mut changed_files = snapshot
        .changed_files
        .iter()
        .filter(|change| diff_change_matches_scope(change, request.scope))
        .cloned()
        .collect::<Vec<_>>();
    let path = relative_path
        .as_ref()
        .map(|path| relative_path_string(path));
    let diff_args = workspace_diff_args(request.scope, path.as_deref(), snapshot.commit.is_some());
    let diff = run_workspace_process(
        &workspace_root,
        "git",
        &diff_args,
        Duration::from_secs(10),
        WORKSPACE_DIFF_BYTES,
        4_096,
    )
    .await;
    let mut diff_text = diff.stdout;
    if !diff.stderr.trim().is_empty() {
        if !diff_text.is_empty() {
            diff_text.push('\n');
        }
        diff_text.push_str(&diff.stderr);
    }
    let mut diff_truncated = diff.stdout_truncated || diff.stderr_truncated;
    if request.scope == WorkspaceDiffScope::All && snapshot.commit.is_none() {
        let staged = run_workspace_process(
            &workspace_root,
            "git",
            &workspace_diff_args(WorkspaceDiffScope::Staged, path.as_deref(), false),
            Duration::from_secs(10),
            WORKSPACE_DIFF_BYTES,
            4_096,
        )
        .await;
        let mut staged_text = staged.stdout;
        if !staged.stderr.trim().is_empty() {
            staged_text.push_str(&staged.stderr);
        }
        if !staged_text.is_empty() {
            staged_text.push_str(&diff_text);
            diff_text = staged_text;
        }
        diff_truncated |= staged.stdout_truncated || staged.stderr_truncated;
    }
    let binary = relative_path
        .as_ref()
        .is_some_and(|path| binary_extension(path))
        || diff_text.contains("Binary files ");
    if binary {
        if let Some(path) = path.as_deref() {
            if let Some(change) = changed_files.iter_mut().find(|change| change.path == path) {
                change.binary = true;
            }
        }
    }
    let (original, modified) = if let Some(relative_path) = &relative_path {
        workspace_diff_contents(
            &workspace_root,
            relative_path,
            request.scope,
            &snapshot,
            binary,
        )
        .await?
    } else {
        (None, None)
    };
    let summary = format!(
        "{} changed · {} staged · {} unstaged · {} untracked · {} conflicted",
        changed_files.len(),
        snapshot.staged_count,
        snapshot.unstaged_count,
        snapshot.untracked_count,
        snapshot.conflicted_count,
    );
    let summary_truncated = snapshot.truncated;
    let diff_id = format!("workspace-diff-{}", command.command_id.as_str());
    let hunks = parse_workspace_diff_hunks(&diff_id, &diff_text);
    Ok(WorkspaceDiffResponse {
        placement_id,
        diff_id,
        git_snapshot: snapshot,
        summary,
        diff: diff_text,
        scope: request.scope,
        path,
        changed_files,
        hunks,
        original,
        modified,
        binary,
        summary_truncated,
        diff_truncated,
        generated_at: Utc::now(),
    })
}

pub(crate) fn parse_workspace_diff_hunks(diff_id: &str, diff: &str) -> Vec<WorkspaceDiffHunk> {
    let mut hunks = Vec::new();
    let mut header: Option<String> = None;
    let mut patch = String::new();
    for line in diff.lines() {
        if hunks.len() >= MAX_WORKSPACE_DIFF_HUNKS {
            break;
        }
        if line.starts_with("@@ ") {
            if let Some(header) = header.take() {
                hunks.push(WorkspaceDiffHunk {
                    hunk_id: format!("{diff_id}:hunk-{}", hunks.len() + 1),
                    header,
                    patch: std::mem::take(&mut patch),
                });
            }
            header = Some(line.to_owned());
            patch.push_str(line);
            patch.push('\n');
        } else if header.is_some() {
            if line.starts_with("diff --git ") {
                if let Some(header) = header.take() {
                    hunks.push(WorkspaceDiffHunk {
                        hunk_id: format!("{diff_id}:hunk-{}", hunks.len() + 1),
                        header,
                        patch: std::mem::take(&mut patch),
                    });
                }
            } else {
                patch.push_str(line);
                patch.push('\n');
            }
        }
    }
    if hunks.len() < MAX_WORKSPACE_DIFF_HUNKS {
        if let Some(header) = header {
            hunks.push(WorkspaceDiffHunk {
                hunk_id: format!("{diff_id}:hunk-{}", hunks.len() + 1),
                header,
                patch,
            });
        }
    }
    hunks
}

fn diff_change_matches_scope(change: &GitChangedFile, scope: WorkspaceDiffScope) -> bool {
    match scope {
        WorkspaceDiffScope::All => true,
        WorkspaceDiffScope::Staged => change.index_status.is_some(),
        WorkspaceDiffScope::Unstaged => change.worktree_status.is_some(),
    }
}

fn workspace_diff_args(
    scope: WorkspaceDiffScope,
    path: Option<&str>,
    has_head: bool,
) -> Vec<String> {
    let mut args = vec![
        "diff".to_owned(),
        "--no-ext-diff".to_owned(),
        "--no-color".to_owned(),
        "--find-renames".to_owned(),
    ];
    match scope {
        WorkspaceDiffScope::All if has_head => args.push("HEAD".to_owned()),
        WorkspaceDiffScope::All | WorkspaceDiffScope::Unstaged => {}
        WorkspaceDiffScope::Staged => args.push("--cached".to_owned()),
    }
    args.push("--".to_owned());
    if let Some(path) = path {
        args.push(path.to_owned());
    }
    args
}

async fn workspace_diff_contents(
    workspace_root: &Path,
    relative_path: &Path,
    scope: WorkspaceDiffScope,
    snapshot: &GitWorkspaceSnapshot,
    binary: bool,
) -> Result<(Option<String>, Option<String>), WorkspaceInspectError> {
    if binary {
        return Ok((None, None));
    }
    let path = relative_path_string(relative_path);
    let change = snapshot
        .changed_files
        .iter()
        .find(|change| change.path == path);
    let untracked =
        change.is_some_and(|change| change.worktree_status == Some(GitChangeKind::Untracked));
    let original = if untracked {
        Some(String::new())
    } else {
        let source_path = change
            .and_then(|change| change.previous_path.as_deref())
            .unwrap_or(&path);
        let spec = match scope {
            WorkspaceDiffScope::Unstaged => format!(":{source_path}"),
            WorkspaceDiffScope::All | WorkspaceDiffScope::Staged => {
                format!("HEAD:{source_path}")
            }
        };
        git_show_text(workspace_root, &spec).await
    };
    let modified = match scope {
        WorkspaceDiffScope::Staged => git_show_text(workspace_root, &format!(":{path}")).await,
        WorkspaceDiffScope::All | WorkspaceDiffScope::Unstaged => {
            read_workspace_diff_file(workspace_root, relative_path).await?
        }
    };
    Ok((
        original.or_else(|| Some(String::new())),
        modified.or_else(|| Some(String::new())),
    ))
}

async fn git_show_text(workspace_root: &Path, spec: &str) -> Option<String> {
    let output = run_workspace_process(
        workspace_root,
        "git",
        &[
            "show".to_owned(),
            "--no-textconv".to_owned(),
            spec.to_owned(),
        ],
        Duration::from_secs(10),
        MAX_WORKSPACE_TEXT_BYTES as usize,
        4_096,
    )
    .await;
    output.success.then_some(output.stdout)
}

async fn read_workspace_diff_file(
    workspace_root: &Path,
    relative_path: &Path,
) -> Result<Option<String>, WorkspaceInspectError> {
    let workspace_root = workspace_root.to_owned();
    let relative_path = relative_path.to_owned();
    tokio::task::spawn_blocking(move || {
        let target = match resolve_existing_workspace_path(&workspace_root, &relative_path) {
            Ok(target) => target,
            Err(error) if error.code == "workspace.path_missing" => return Ok(None),
            Err(error) => return Err(error),
        };
        let metadata = fs::metadata(&target).map_err(|error| {
            WorkspaceInspectError::new(
                "workspace.metadata_failed",
                format!("Failed to inspect diff file: {error}"),
            )
        })?;
        if !metadata.is_file() || metadata.len() > MAX_WORKSPACE_TEXT_BYTES {
            return Ok(None);
        }
        let bytes = fs::read(target).map_err(|error| {
            WorkspaceInspectError::new(
                "workspace.read_failed",
                format!("Failed to read diff file: {error}"),
            )
        })?;
        if bytes.contains(&0) {
            return Ok(None);
        }
        String::from_utf8(bytes).map(Some).map_err(|_| {
            WorkspaceInspectError::new(
                "workspace.diff_binary_file",
                "Workspace diff content is not UTF-8 text",
            )
        })
    })
    .await
    .map_err(|error| {
        WorkspaceInspectError::new(
            "workspace.diff_read_failed",
            format!("Workspace diff reader failed: {error}"),
        )
    })?
}

pub(crate) fn workspace_command_payload<T: for<'de> Deserialize<'de>>(
    command: &CommandEnvelope,
) -> Result<T, WorkspaceInspectError> {
    command.payload.workspace_request().ok_or_else(|| {
        WorkspaceInspectError::new(
            "workspace.invalid_payload",
            "Workspace command payload does not match its command kind",
        )
    })
}

pub(crate) fn validate_workspace_command_request(
    request: &WorkspaceCommandRunRequest,
) -> Result<(), WorkspaceInspectError> {
    let command = request.command.trim();
    if command.is_empty() {
        return Err(WorkspaceInspectError::new(
            "workspace.command_required",
            "Workspace command requires an executable name",
        ));
    }
    if command.chars().count() > MAX_WORKSPACE_COMMAND_ARG_CHARS
        || command.contains('\0')
        || command.contains('/')
        || command.contains('\\')
    {
        return Err(WorkspaceInspectError::new(
            "workspace.command_invalid",
            "Workspace command executable must be a program name, not a path",
        ));
    }
    if !ALLOWED_WORKSPACE_COMMANDS.contains(&command) {
        return Err(WorkspaceInspectError::new(
            "workspace.command_not_allowed",
            format!("Workspace command `{command}` is not allowed by node policy"),
        ));
    }
    if request.args.len() > MAX_WORKSPACE_COMMAND_ARGS {
        return Err(WorkspaceInspectError::new(
            "workspace.command_too_many_args",
            format!(
                "Workspace commands accept at most {} arguments",
                MAX_WORKSPACE_COMMAND_ARGS
            ),
        ));
    }
    if request
        .args
        .iter()
        .any(|arg| arg.contains('\0') || arg.chars().count() > MAX_WORKSPACE_COMMAND_ARG_CHARS)
    {
        return Err(WorkspaceInspectError::new(
            "workspace.command_arg_invalid",
            format!(
                "Workspace command arguments are limited to {} characters",
                MAX_WORKSPACE_COMMAND_ARG_CHARS
            ),
        ));
    }
    Ok(())
}

pub(crate) struct WorkspaceProcessOutput {
    pub(crate) exit_code: Option<i32>,
    pub(crate) success: bool,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) stdout_truncated: bool,
    pub(crate) stderr_truncated: bool,
    pub(crate) duration_ms: u64,
    pub(crate) started_at: DateTime<Utc>,
    pub(crate) completed_at: DateTime<Utc>,
}

pub(crate) async fn run_workspace_process(
    workspace_root: &Path,
    command_name: &str,
    args: &[String],
    timeout_duration: Duration,
    stdout_cap: usize,
    stderr_cap: usize,
) -> WorkspaceProcessOutput {
    let started_at = Utc::now();
    let started = Instant::now();
    let mut command = TokioCommand::new(command_name);
    command
        .args(args)
        .current_dir(workspace_root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return WorkspaceProcessOutput {
                exit_code: None,
                success: false,
                stdout: String::new(),
                stderr: format!("Failed to start `{command_name}`: {error}"),
                stdout_truncated: false,
                stderr_truncated: false,
                duration_ms: duration_millis(started),
                started_at,
                completed_at: Utc::now(),
            };
        }
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_task = tokio::spawn(read_capped_process_output(stdout, stdout_cap));
    let stderr_task = tokio::spawn(read_capped_process_output(stderr, stderr_cap));
    let wait_result = timeout(timeout_duration, child.wait()).await;
    let completed_at = Utc::now();
    let duration_ms = duration_millis(started);
    match wait_result {
        Ok(Ok(status)) => {
            let (stdout, stdout_truncated) = join_capped_output(stdout_task).await;
            let (stderr, stderr_truncated) = join_capped_output(stderr_task).await;
            WorkspaceProcessOutput {
                exit_code: status.code(),
                success: status.success(),
                stdout,
                stderr,
                stdout_truncated,
                stderr_truncated,
                duration_ms,
                started_at,
                completed_at,
            }
        }
        Ok(Err(error)) => WorkspaceProcessOutput {
            exit_code: None,
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to start `{command_name}`: {error}"),
            stdout_truncated: false,
            stderr_truncated: false,
            duration_ms,
            started_at,
            completed_at,
        },
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            let (stdout, stdout_truncated) = join_capped_output(stdout_task).await;
            let (mut stderr, stderr_truncated) = join_capped_output(stderr_task).await;
            let timeout_message = format!(
                "`{command_name}` timed out after {} seconds",
                timeout_duration.as_secs()
            );
            if stderr.trim().is_empty() {
                stderr = timeout_message;
            } else {
                stderr.push('\n');
                stderr.push_str(&timeout_message);
            }
            WorkspaceProcessOutput {
                exit_code: None,
                success: false,
                stdout,
                stderr,
                stdout_truncated,
                stderr_truncated,
                duration_ms,
                started_at,
                completed_at,
            }
        }
    }
}

pub(crate) async fn read_capped_process_output<R>(
    reader: Option<R>,
    cap: usize,
) -> std::io::Result<(String, bool)>
where
    R: AsyncRead + Unpin,
{
    let Some(mut reader) = reader else {
        return Ok((String::new(), false));
    };
    let mut buffer = [0_u8; 8192];
    let mut collected = Vec::with_capacity(cap.min(8192));
    let mut truncated = false;
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let remaining = cap.saturating_sub(collected.len());
        if remaining > 0 {
            let keep = read.min(remaining);
            collected.extend_from_slice(&buffer[..keep]);
            truncated |= keep < read;
        } else {
            truncated = true;
        }
    }
    Ok((String::from_utf8_lossy(&collected).into_owned(), truncated))
}

pub(crate) async fn join_capped_output(
    task: tokio::task::JoinHandle<std::io::Result<(String, bool)>>,
) -> (String, bool) {
    match task.await {
        Ok(Ok(output)) => output,
        Ok(Err(error)) => (format!("failed to read process output: {error}"), false),
        Err(error) => (
            format!("failed to join process output reader: {error}"),
            false,
        ),
    }
}

pub(crate) fn duration_millis(started: Instant) -> u64 {
    let millis = started.elapsed().as_millis();
    if millis > u128::from(u64::MAX) {
        u64::MAX
    } else {
        millis as u64
    }
}

#[derive(Debug)]
pub(crate) struct WorkspaceInspectError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

impl WorkspaceInspectError {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub(crate) fn into_payload(self) -> JsonValue {
        JsonValue(serde_json::json!({
            "error_code": self.code,
            "message": self.message,
        }))
    }
}

pub(crate) fn build_workspace_tree_response(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> Result<WorkspaceTreeResponse, WorkspaceInspectError> {
    let placement_id = workspace_command_placement_id(command)?;
    let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
    let requested_path = command_payload_str(command, "path").unwrap_or(".");
    let relative_path = safe_workspace_relative_path(requested_path)?;
    let root_path = match resolve_existing_workspace_path(&workspace_root, &relative_path) {
        Ok(path) => path,
        Err(error) if error.code == "workspace.path_missing" => workspace_root.join(&relative_path),
        Err(error) => return Err(error),
    };
    let mut root = workspace_tree_entry(&root_path, &relative_path);
    let mut truncated = false;
    let mut total_entries = None;
    if root.kind == WorkspaceEntryKind::Directory && root.status == WorkspaceEntryStatus::Directory
    {
        match std::fs::read_dir(&root_path) {
            Ok(read_dir) => {
                let mut entries = read_dir.filter_map(Result::ok).collect::<Vec<_>>();
                entries.sort_by(|left, right| {
                    let left_is_dir = left.file_type().is_ok_and(|kind| kind.is_dir());
                    let right_is_dir = right.file_type().is_ok_and(|kind| kind.is_dir());
                    right_is_dir
                        .cmp(&left_is_dir)
                        .then_with(|| left.file_name().cmp(&right.file_name()))
                });
                total_entries = Some(entries.len() as u64);
                truncated = entries.len() > MAX_WORKSPACE_DIRECTORY_ENTRIES;
                root.children = entries
                    .into_iter()
                    .take(MAX_WORKSPACE_DIRECTORY_ENTRIES)
                    .map(|entry| {
                        let child_relative_path = relative_path.join(entry.file_name());
                        workspace_tree_entry(&entry.path(), &child_relative_path)
                    })
                    .collect();
            }
            Err(error) if error.kind() == ErrorKind::PermissionDenied => {
                root.status = WorkspaceEntryStatus::PermissionDenied;
                root.expandable = false;
            }
            Err(_) => {
                root.status = WorkspaceEntryStatus::Error;
                root.expandable = false;
            }
        }
    }
    Ok(WorkspaceTreeResponse {
        placement_id,
        root,
        truncated,
        total_entries,
        generated_at: Utc::now(),
    })
}

pub(crate) fn build_workspace_file_response(
    config: &NodeConfig,
    command: &CommandEnvelope,
) -> Result<WorkspaceFileContentResponse, WorkspaceInspectError> {
    let placement_id = workspace_command_placement_id(command)?;
    let workspace_root = canonical_workspace_root(config, workspace_command_path(command)?)?;
    let requested_path = command_payload_str(command, "path").unwrap_or(".");
    let relative_path = safe_workspace_relative_path(requested_path)?;
    let response_path = relative_path_string(&relative_path);
    let target_path = match resolve_existing_workspace_path(&workspace_root, &relative_path) {
        Ok(path) => path,
        Err(error) if error.code == "workspace.path_missing" => {
            return Ok(workspace_file_status_response(
                placement_id,
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::Missing,
                None,
                None,
                None,
            ));
        }
        Err(error) => return Err(error),
    };
    let metadata = match std::fs::symlink_metadata(&target_path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            return Ok(workspace_file_status_response(
                placement_id,
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::PermissionDenied,
                None,
                None,
                None,
            ));
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Ok(workspace_file_status_response(
                placement_id,
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::Missing,
                None,
                None,
                None,
            ));
        }
        Err(error) => {
            return Err(WorkspaceInspectError::new(
                "workspace.metadata_failed",
                format!("Failed to inspect {response_path}: {error}"),
            ));
        }
    };
    let kind = workspace_entry_kind(&metadata);
    let modified_at = metadata_modified_at(&metadata);
    if metadata.file_type().is_symlink() {
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            WorkspaceEntryStatus::Symlink,
            None,
            modified_at,
            None,
        ));
    }
    if !metadata.is_file() {
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            WorkspaceEntryStatus::NotFile,
            Some(metadata.len()),
            modified_at,
            None,
        ));
    }
    if generated_or_ignored_status(&relative_path).is_some() {
        let status =
            generated_or_ignored_status(&relative_path).unwrap_or(WorkspaceEntryStatus::Generated);
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            status,
            Some(metadata.len()),
            modified_at,
            None,
        ));
    }
    if metadata.len() > MAX_WORKSPACE_TEXT_BYTES {
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            WorkspaceEntryStatus::Large,
            Some(metadata.len()),
            modified_at,
            None,
        ));
    }
    if binary_extension(&relative_path) {
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            WorkspaceEntryStatus::Binary,
            Some(metadata.len()),
            modified_at,
            None,
        ));
    }

    let bytes = match std::fs::read(&target_path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            return Ok(workspace_file_status_response(
                placement_id,
                relative_path,
                kind,
                WorkspaceEntryStatus::PermissionDenied,
                Some(metadata.len()),
                modified_at,
                None,
            ));
        }
        Err(error) => {
            return Err(WorkspaceInspectError::new(
                "workspace.read_failed",
                format!("Failed to read {response_path}: {error}"),
            ));
        }
    };
    if bytes.contains(&0) {
        return Ok(workspace_file_status_response(
            placement_id,
            relative_path,
            kind,
            WorkspaceEntryStatus::Binary,
            Some(metadata.len()),
            modified_at,
            None,
        ));
    }
    let content = match String::from_utf8(bytes) {
        Ok(content) => content,
        Err(_) => {
            return Ok(workspace_file_status_response(
                placement_id,
                relative_path,
                kind,
                WorkspaceEntryStatus::Binary,
                Some(metadata.len()),
                modified_at,
                None,
            ));
        }
    };

    Ok(workspace_file_status_response(
        placement_id,
        relative_path,
        kind,
        WorkspaceEntryStatus::Readable,
        Some(metadata.len()),
        modified_at,
        Some(content),
    ))
}

pub(crate) fn workspace_file_status_response(
    placement_id: ProjectPlacementId,
    relative_path: PathBuf,
    kind: WorkspaceEntryKind,
    status: WorkspaceEntryStatus,
    byte_len: Option<u64>,
    modified_at: Option<DateTime<Utc>>,
    content: Option<String>,
) -> WorkspaceFileContentResponse {
    let path = relative_path_string(&relative_path);
    WorkspaceFileContentResponse {
        placement_id,
        path: path.clone(),
        metadata: WorkspaceEntry {
            name: workspace_entry_name(&relative_path),
            path,
            kind,
            status,
            classification: workspace_entry_classification(&relative_path),
            expandable: false,
            byte_len,
            modified_at,
            children: vec![],
        },
        content,
        truncated: false,
        generated_at: Utc::now(),
    }
}

pub(crate) fn workspace_command_placement_id(
    command: &CommandEnvelope,
) -> Result<ProjectPlacementId, WorkspaceInspectError> {
    command
        .target
        .project_placement_id()
        .cloned()
        .ok_or_else(|| {
            WorkspaceInspectError::new(
                "workspace.placement_missing",
                "Workspace inspector command is missing a placement id",
            )
        })
}

pub(crate) fn workspace_command_path(
    command: &CommandEnvelope,
) -> Result<&str, WorkspaceInspectError> {
    command_payload_str(command, "workspace_path")
        .filter(|path| !path.trim().is_empty())
        .ok_or_else(|| {
            WorkspaceInspectError::new(
                "workspace.path_required",
                "Workspace inspector command is missing a workspace path",
            )
        })
}

pub(crate) fn canonical_workspace_root(
    config: &NodeConfig,
    workspace_path: &str,
) -> Result<PathBuf, WorkspaceInspectError> {
    canonical_workspace_root_for_allowed_paths(&config.workspace_paths, workspace_path)
}

pub(crate) fn canonical_workspace_root_for_allowed_paths(
    allowed_paths: &[PathBuf],
    workspace_path: &str,
) -> Result<PathBuf, WorkspaceInspectError> {
    let root = std::fs::canonicalize(workspace_path).map_err(|error| {
        let code = if error.kind() == ErrorKind::NotFound {
            "workspace.root_missing"
        } else if error.kind() == ErrorKind::PermissionDenied {
            "workspace.root_permission_denied"
        } else {
            "workspace.root_invalid"
        };
        WorkspaceInspectError::new(
            code,
            format!("Workspace root {workspace_path} is not readable: {error}"),
        )
    })?;
    if !root.is_dir() {
        return Err(WorkspaceInspectError::new(
            "workspace.root_not_directory",
            "Workspace root is not a directory",
        ));
    }
    if !canonical_workspace_path_allowed_roots(allowed_paths, &root) {
        return Err(WorkspaceInspectError::new(
            "workspace.outside_allowed_roots",
            "Workspace root is outside the node allowed roots",
        ));
    }
    Ok(root)
}

pub(crate) fn canonical_workspace_path_allowed(config: &NodeConfig, workspace_root: &Path) -> bool {
    canonical_workspace_path_allowed_roots(&config.workspace_paths, workspace_root)
}

pub(crate) fn canonical_workspace_path_allowed_roots(
    allowed_paths: &[PathBuf],
    workspace_root: &Path,
) -> bool {
    !allowed_paths.is_empty()
        && allowed_paths.iter().any(|allowed_root| {
            std::fs::canonicalize(allowed_root)
                .map(|root| workspace_root.starts_with(root))
                .unwrap_or(false)
        })
}

pub(crate) fn safe_workspace_relative_path(path: &str) -> Result<PathBuf, WorkspaceInspectError> {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "." {
        return Ok(PathBuf::new());
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err(WorkspaceInspectError::new(
            "workspace.absolute_path",
            "Workspace inspector paths must be relative",
        ));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Normal(value) => normalized.push(value),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => {
                return Err(WorkspaceInspectError::new(
                    "workspace.path_escape",
                    "Workspace inspector paths cannot leave the workspace",
                ));
            }
        }
    }
    Ok(normalized)
}

pub(crate) fn resolve_existing_workspace_path(
    workspace_root: &Path,
    relative_path: &Path,
) -> Result<PathBuf, WorkspaceInspectError> {
    let mut current = workspace_root.to_path_buf();
    if relative_path.as_os_str().is_empty() {
        return Ok(current);
    }
    for component in relative_path.components() {
        let std::path::Component::Normal(segment) = component else {
            return Err(WorkspaceInspectError::new(
                "workspace.path_escape",
                "Workspace inspector paths cannot leave the workspace",
            ));
        };
        current.push(segment);
        let metadata = std::fs::symlink_metadata(&current).map_err(|error| {
            let code = if error.kind() == ErrorKind::NotFound {
                "workspace.path_missing"
            } else if error.kind() == ErrorKind::PermissionDenied {
                "workspace.permission_denied"
            } else {
                "workspace.metadata_failed"
            };
            WorkspaceInspectError::new(
                code,
                format!("Failed to inspect {}: {error}", relative_path.display()),
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Ok(current);
        }
        let canonical = std::fs::canonicalize(&current).map_err(|error| {
            WorkspaceInspectError::new(
                "workspace.canonicalize_failed",
                format!("Failed to resolve {}: {error}", relative_path.display()),
            )
        })?;
        if !canonical.starts_with(workspace_root) {
            return Err(WorkspaceInspectError::new(
                "workspace.path_escape",
                "Workspace inspector path resolves outside the workspace",
            ));
        }
        current = canonical;
    }
    Ok(current)
}

pub(crate) fn workspace_tree_entry(path: &Path, relative_path: &Path) -> WorkspaceEntry {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::PermissionDenied => {
            return workspace_entry_for_status(
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::PermissionDenied,
                None,
                None,
            );
        }
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return workspace_entry_for_status(
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::Missing,
                None,
                None,
            );
        }
        Err(_) => {
            return workspace_entry_for_status(
                relative_path,
                WorkspaceEntryKind::Other,
                WorkspaceEntryStatus::Error,
                None,
                None,
            );
        }
    };
    let kind = workspace_entry_kind(&metadata);
    let status = workspace_status_for_entry(relative_path, &metadata);

    WorkspaceEntry {
        name: workspace_entry_name(relative_path),
        path: relative_path_string(relative_path),
        kind,
        status,
        classification: workspace_entry_classification(relative_path),
        expandable: metadata.is_dir(),
        byte_len: metadata.is_file().then_some(metadata.len()),
        modified_at: metadata_modified_at(&metadata),
        children: vec![],
    }
}

pub(crate) fn workspace_entry_for_status(
    relative_path: &Path,
    kind: WorkspaceEntryKind,
    status: WorkspaceEntryStatus,
    byte_len: Option<u64>,
    modified_at: Option<DateTime<Utc>>,
) -> WorkspaceEntry {
    WorkspaceEntry {
        name: workspace_entry_name(relative_path),
        path: relative_path_string(relative_path),
        kind,
        status,
        classification: WorkspaceEntryClassification::Normal,
        expandable: false,
        byte_len,
        modified_at,
        children: vec![],
    }
}

pub(crate) fn workspace_entry_kind(metadata: &std::fs::Metadata) -> WorkspaceEntryKind {
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        WorkspaceEntryKind::Directory
    } else if file_type.is_file() {
        WorkspaceEntryKind::File
    } else if file_type.is_symlink() {
        WorkspaceEntryKind::Symlink
    } else {
        WorkspaceEntryKind::Other
    }
}

pub(crate) fn workspace_status_for_entry(
    relative_path: &Path,
    metadata: &std::fs::Metadata,
) -> WorkspaceEntryStatus {
    if metadata.file_type().is_symlink() {
        return WorkspaceEntryStatus::Symlink;
    }
    if metadata.is_dir() {
        return WorkspaceEntryStatus::Directory;
    }
    if metadata.is_file() {
        if metadata.len() > MAX_WORKSPACE_TEXT_BYTES {
            return WorkspaceEntryStatus::Large;
        }
        if binary_extension(relative_path) {
            return WorkspaceEntryStatus::Binary;
        }
        return WorkspaceEntryStatus::Readable;
    }
    WorkspaceEntryStatus::Error
}

pub(crate) fn workspace_entry_classification(relative_path: &Path) -> WorkspaceEntryClassification {
    match generated_or_ignored_status(relative_path) {
        Some(WorkspaceEntryStatus::Generated) => WorkspaceEntryClassification::Generated,
        Some(WorkspaceEntryStatus::Ignored) => WorkspaceEntryClassification::Ignored,
        _ => WorkspaceEntryClassification::Normal,
    }
}

pub(crate) fn generated_or_ignored_status(relative_path: &Path) -> Option<WorkspaceEntryStatus> {
    let mut generated = None;
    for component in relative_path.components() {
        let std::path::Component::Normal(value) = component else {
            continue;
        };
        let Some(name) = value.to_str() else {
            continue;
        };
        match name {
            ".git" | ".hg" | ".svn" | ".DS_Store" => return Some(WorkspaceEntryStatus::Ignored),
            ".local" | "node_modules" | "target" | "dist" | "build" | "coverage" | ".next"
            | ".turbo" | ".vite" => generated = Some(WorkspaceEntryStatus::Generated),
            _ => {}
        }
    }
    generated
}

pub(crate) fn workspace_entry_status_label(status: WorkspaceEntryStatus) -> &'static str {
    match status {
        WorkspaceEntryStatus::Readable => "readable",
        WorkspaceEntryStatus::Directory => "directory",
        WorkspaceEntryStatus::Large => "large",
        WorkspaceEntryStatus::Binary => "binary",
        WorkspaceEntryStatus::Ignored => "ignored",
        WorkspaceEntryStatus::Generated => "generated",
        WorkspaceEntryStatus::PermissionDenied => "permission-denied",
        WorkspaceEntryStatus::OutsideWorkspace => "outside-workspace",
        WorkspaceEntryStatus::Missing => "missing",
        WorkspaceEntryStatus::NotFile => "not-file",
        WorkspaceEntryStatus::NotDirectory => "not-directory",
        WorkspaceEntryStatus::Symlink => "symlink",
        WorkspaceEntryStatus::Error => "error",
    }
}

pub(crate) fn binary_extension(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
        return false;
    };
    matches!(
        extension.to_ascii_lowercase().as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "ico"
            | "pdf"
            | "zip"
            | "gz"
            | "tar"
            | "tgz"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "wasm"
            | "sqlite"
            | "db"
            | "bin"
            | "exe"
            | "dylib"
            | "so"
            | "dll"
    )
}

pub(crate) fn workspace_entry_name(relative_path: &Path) -> String {
    if relative_path.as_os_str().is_empty() {
        return ".".to_owned();
    }
    relative_path
        .file_name()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_owned())
}

pub(crate) fn relative_path_string(relative_path: &Path) -> String {
    if relative_path.as_os_str().is_empty() {
        return ".".to_owned();
    }
    relative_path
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub(crate) fn metadata_modified_at(metadata: &std::fs::Metadata) -> Option<DateTime<Utc>> {
    metadata.modified().ok().map(DateTime::<Utc>::from)
}

pub(crate) fn placement_event_for_command(
    command: &CommandEnvelope,
    placement_seqs: &mut HashMap<String, i64>,
    project_placement_id: ProjectPlacementId,
    kind: EventKind,
    payload: serde_json::Value,
) -> EventEnvelope {
    let seq = next_placement_seq(placement_seqs, &project_placement_id);
    EventEnvelope {
        event_id: EventId::new(),
        command_id: Some(command.command_id.clone()),
        correlation_id: Some(command.correlation_id.clone()),
        actor_ref: ActorRef::Node {
            node_id: command.target.node_id().clone(),
        },
        scope_ref: ScopeRef::Placement {
            project_placement_id,
        },
        node_id: Some(command.target.node_id().clone()),
        runtime_session_id: None,
        session_thread_id: None,
        turn_id: None,
        seq,
        session_projection_seq: None,
        kind,
        happened_at: Utc::now(),
        source_refs: command.source_refs.clone(),
        evidence_refs: vec![],
        cause_refs: command.cause_refs.clone(),
        result_refs: vec![],
        payload: EventPayload::from_json(kind, payload),
    }
}

pub(crate) fn next_placement_seq(
    placement_seqs: &mut HashMap<String, i64>,
    project_placement_id: &ProjectPlacementId,
) -> i64 {
    let entry = placement_seqs
        .entry(project_placement_id.to_string())
        .and_modify(|seq| *seq += 1)
        .or_insert(1);
    *entry
}
