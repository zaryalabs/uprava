//! Node protocol URLs, capability discovery and versioned state paths.

use super::*;

pub(crate) fn control_url(core_url: &Url) -> anyhow::Result<Url> {
    let mut url = core_url
        .join("/api/v1/node/control")
        .context("control URL should be valid")?;
    let scheme = match url.scheme() {
        "http" => "ws",
        "https" => "wss",
        other => anyhow::bail!("unsupported Core URL scheme `{other}`"),
    };
    url.set_scheme(scheme)
        .map_err(|_| anyhow::anyhow!("failed to set control URL scheme"))?;
    Ok(url)
}

pub(crate) fn capabilities(config: &NodeConfig) -> Vec<CapabilitySummary> {
    let codex_available = command_available(&config.codex_binary);
    vec![
        CapabilitySummary {
            key: "provider.codex".to_owned(),
            value: CapabilityValue::Provider {
                available: codex_available,
                configured: true,
                mode: "exec".to_owned(),
                timeout_seconds: Some(config.codex_timeout.as_secs()),
                unavailable_reason: (!codex_available).then(|| "binary_not_found".to_owned()),
            },
        },
        CapabilitySummary {
            key: "workspace.validation".to_owned(),
            value: CapabilityValue::WorkspaceValidation {
                mode: "explicit_path".to_owned(),
            },
        },
    ]
}

pub(crate) fn command_available(binary: &str) -> bool {
    let path = Path::new(binary);
    if path.components().count() > 1 || path.is_absolute() {
        return path.is_file();
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    command_available_in_search_path(binary, &paths)
}

pub(crate) fn command_available_in_search_path(
    binary: &str,
    search_path: &std::ffi::OsStr,
) -> bool {
    std::env::split_paths(search_path).any(|directory| directory.join(binary).is_file())
}

pub(crate) fn validate_workspace(path: &Path) -> WorkspaceSnapshot {
    let display_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("workspace")
        .to_owned();
    let (state, resource_badges) = if !path.exists() {
        (
            PlacementState::Missing,
            vec![ResourceBadge {
                kind: "missing_workspace".to_owned(),
                severity: WarningSeverity::HardBlock,
                label: "Workspace missing".to_owned(),
            }],
        )
    } else if !path.is_dir() {
        (
            PlacementState::Missing,
            vec![ResourceBadge {
                kind: "workspace_not_directory".to_owned(),
                severity: WarningSeverity::HardBlock,
                label: "Workspace is not a directory".to_owned(),
            }],
        )
    } else if std::fs::metadata(path)
        .map(|metadata| metadata.permissions().readonly())
        .unwrap_or(false)
    {
        (
            PlacementState::ReadOnly,
            vec![ResourceBadge {
                kind: "read_only_workspace".to_owned(),
                severity: WarningSeverity::HardBlock,
                label: "Read-only workspace".to_owned(),
            }],
        )
    } else {
        (PlacementState::Validated, resource_warnings(path))
    };

    WorkspaceSnapshot {
        display_name,
        workspace_path: path.display().to_string(),
        state,
        resource_badges,
        last_validated_at: chrono::Utc::now(),
    }
}

pub(crate) fn resource_warnings(path: &Path) -> Vec<ResourceBadge> {
    let mut badges = Vec::new();
    if !path.join(".git").exists() {
        return badges;
    }

    badges.push(ResourceBadge {
        kind: "git_workspace".to_owned(),
        severity: WarningSeverity::Info,
        label: "Git workspace".to_owned(),
    });
    if let Some(status) = git_status_snapshot(path) {
        badges.extend(git_status_badges(&status));
    } else {
        badges.push(ResourceBadge {
            kind: "git_snapshot_unavailable".to_owned(),
            severity: WarningSeverity::Warning,
            label: "Git snapshot unavailable".to_owned(),
        });
    }
    badges
}

pub(crate) fn git_status_snapshot(path: &Path) -> Option<String> {
    let output = StdCommand::new("git")
        .arg("-C")
        .arg(path)
        .arg("status")
        .arg("--porcelain=v1")
        .arg("--branch")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

pub(crate) fn git_status_badges(status: &str) -> Vec<ResourceBadge> {
    let mut branch = None;
    let mut dirty_count = 0usize;
    let mut untracked_count = 0usize;
    let mut ahead_count = 0usize;
    let mut behind_count = 0usize;

    for line in status.lines() {
        if let Some(summary) = line.strip_prefix("## ") {
            branch = git_branch_label(summary);
            ahead_count = git_tracking_count(summary, "ahead");
            behind_count = git_tracking_count(summary, "behind");
        } else if !line.trim().is_empty() {
            dirty_count += 1;
            if line.starts_with("?? ") {
                untracked_count += 1;
            }
        }
    }

    let mut badges = Vec::new();
    if let Some(branch) = branch {
        badges.push(ResourceBadge {
            kind: "git_branch".to_owned(),
            severity: WarningSeverity::Info,
            label: format!("Git branch: {branch}"),
        });
    }
    if dirty_count > 0 {
        badges.push(ResourceBadge {
            kind: "dirty_workspace".to_owned(),
            severity: WarningSeverity::Warning,
            label: dirty_workspace_label(dirty_count, untracked_count),
        });
    }
    if behind_count > 0 {
        badges.push(ResourceBadge {
            kind: "branch_behind".to_owned(),
            severity: WarningSeverity::Warning,
            label: format!("Branch is behind upstream by {behind_count} commit(s)"),
        });
    }
    if ahead_count > 0 {
        badges.push(ResourceBadge {
            kind: "branch_ahead".to_owned(),
            severity: WarningSeverity::Info,
            label: format!("Branch is ahead of upstream by {ahead_count} commit(s)"),
        });
    }
    badges
}

pub(crate) fn git_branch_label(summary: &str) -> Option<String> {
    let without_upstream = summary.split("...").next().unwrap_or(summary);
    let without_tracking = without_upstream
        .split(" [")
        .next()
        .unwrap_or(without_upstream)
        .trim();
    (!without_tracking.is_empty()).then(|| without_tracking.to_owned())
}

pub(crate) fn git_tracking_count(summary: &str, key: &str) -> usize {
    let Some((_, tracking)) = summary.split_once('[') else {
        return 0;
    };
    let tracking = tracking.strip_suffix(']').unwrap_or(tracking);
    tracking
        .split(',')
        .map(str::trim)
        .find_map(|part| {
            part.strip_prefix(key)
                .and_then(|value| value.trim().parse::<usize>().ok())
        })
        .unwrap_or(0)
}

pub(crate) fn dirty_workspace_label(dirty_count: usize, untracked_count: usize) -> String {
    if untracked_count == 0 {
        return format!("Git workspace has {dirty_count} changed path(s)");
    }
    format!(
        "Git workspace has {dirty_count} changed path(s), including {untracked_count} untracked"
    )
}

pub(crate) fn default_state_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.join(".local")
        .join("share")
        .join("uprava-node")
        .join(NODE_STATE_SLOT)
        .join("node.sqlite")
}

pub(crate) fn is_sqlite_state_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "sqlite")
}

pub(crate) fn legacy_state_path(path: &Path) -> Option<PathBuf> {
    let slot_dir = path.parent()?;
    if slot_dir.file_name()?.to_string_lossy() != NODE_STATE_SLOT {
        return None;
    }
    Some(slot_dir.parent()?.join("node.json"))
}

pub(crate) fn is_versioned_state_path(path: &Path) -> bool {
    path.parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name.to_string_lossy() == NODE_STATE_SLOT)
}
