//! Numbered Core SQLite migrations and checksum metadata.

use super::super::*;

impl AppState {
    pub(crate) async fn migrate(&self) -> Result<(), AppError> {
        self.ensure_compatible_state().await?;
        sqlx::query("pragma foreign_keys = on")
            .execute(&self.pool)
            .await?;
        sqlx::query("pragma busy_timeout = 5000")
            .execute(&self.pool)
            .await?;
        let journal_mode: String = sqlx::query_scalar("pragma journal_mode")
            .fetch_one(&self.pool)
            .await?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            sqlx::query("pragma journal_mode = wal")
                .execute(&self.pool)
                .await?;
        }
        sqlx::query(
            "create table if not exists schema_migrations (version integer primary key, checksum text not null, applied_at text not null)",
        )
        .execute(&self.pool)
        .await?;

        for migration in MIGRATIONS {
            let checksum = migration.checksum();
            let mut connection = self.pool.acquire().await?;
            sqlx::query("begin immediate")
                .execute(&mut *connection)
                .await?;
            let result = async {
                let applied: Option<String> = sqlx::query_scalar(
                    "select checksum from schema_migrations where version = ?1",
                )
                .bind(migration.version)
                .fetch_optional(&mut *connection)
                .await?;
                if let Some(applied_checksum) = applied {
                    if applied_checksum != checksum {
                        return Err(sqlx::Error::Protocol(format!(
                            "migration {} checksum mismatch",
                            migration.version
                        )));
                    }
                    return Ok(());
                }

                for statement in migration.statements {
                    if let Err(error) = sqlx::query(statement).execute(&mut *connection).await {
                        if migration.ignore_duplicate_columns && is_duplicate_column_error(&error) {
                            continue;
                        }
                        return Err(error);
                    }
                }
                sqlx::query(
                    "insert into schema_migrations (version, checksum, applied_at) values (?1, ?2, ?3)",
                )
                .bind(migration.version)
                .bind(&checksum)
                .bind(Utc::now())
                .execute(&mut *connection)
                .await?;
                Ok::<(), sqlx::Error>(())
            }
            .await;
            match result {
                Ok(()) => {
                    sqlx::query("commit").execute(&mut *connection).await?;
                }
                Err(error) => {
                    let _ = sqlx::query("rollback").execute(&mut *connection).await;
                    return Err(error.into());
                }
            }
        }
        Ok(())
    }

    pub(crate) async fn ensure_compatible_state(&self) -> Result<(), AppError> {
        let metadata_table_exists: i64 = sqlx::query_scalar(
            "select count(*) from sqlite_master where type = 'table' and name = 'core_schema_meta'",
        )
        .fetch_one(&self.pool)
        .await?;
        if metadata_table_exists > 0 {
            let metadata_count: i64 = sqlx::query_scalar("select count(*) from core_schema_meta")
                .fetch_one(&self.pool)
                .await?;
            if metadata_count != 1 {
                return Err(incompatible_state_error(format!(
                    "expected exactly one Core schema metadata row, found {metadata_count}"
                )));
            }
            let metadata: Option<(String, i64)> =
                sqlx::query_as("select slot, schema_version from core_schema_meta")
                    .fetch_optional(&self.pool)
                    .await?;
            let Some((slot, schema_version)) = metadata else {
                return Err(incompatible_state_error("core schema metadata is empty"));
            };
            if slot != CORE_STATE_SLOT || schema_version != SCHEMA_VERSION {
                return Err(incompatible_state_error(format!(
                    "expected slot {CORE_STATE_SLOT} schema {SCHEMA_VERSION}, found slot {slot} schema {schema_version}"
                )));
            }
            return Ok(());
        }

        let unexpected_table_count: i64 = sqlx::query_scalar(
            "select count(*) from sqlite_master where type = 'table' and name not like 'sqlite_%' and name not in ('schema_migrations', 'core_schema_meta')",
        )
        .fetch_one(&self.pool)
        .await?;
        let numbered_migration_table_exists: i64 = sqlx::query_scalar(
            "select count(*) from sqlite_master where type = 'table' and name = 'schema_migrations'",
        )
        .fetch_one(&self.pool)
        .await?;
        let applied_migration_count: i64 = if numbered_migration_table_exists == 0 {
            0
        } else {
            sqlx::query_scalar("select count(*) from schema_migrations")
                .fetch_one(&self.pool)
                .await?
        };
        if unexpected_table_count > 0 && applied_migration_count == 0 {
            return Err(incompatible_state_error(
                "unversioned or retained 0.1.x Core state was not modified",
            ));
        }
        if applied_migration_count > 0 {
            let applied: Vec<(i64, String)> =
                sqlx::query_as("select version, checksum from schema_migrations")
                    .fetch_all(&self.pool)
                    .await?;
            for (version, checksum) in applied {
                let Some(migration) = MIGRATIONS
                    .iter()
                    .find(|migration| migration.version == version)
                else {
                    return Err(incompatible_state_error(format!(
                        "unknown numbered migration {version}"
                    )));
                };
                if migration.checksum() != checksum {
                    return Err(incompatible_state_error(format!(
                        "numbered migration {version} checksum mismatch"
                    )));
                }
            }
        }
        Ok(())
    }
}

pub(crate) struct Migration {
    pub(crate) version: i64,
    pub(crate) statements: &'static [&'static str],
    pub(crate) ignore_duplicate_columns: bool,
}

impl Migration {
    pub(crate) fn checksum(&self) -> String {
        let mut digest = Sha256::new();
        digest.update(self.version.to_string().as_bytes());
        digest.update([u8::from(self.ignore_duplicate_columns)]);
        for statement in self.statements {
            digest.update([0]);
            digest.update(statement.as_bytes());
        }
        format!("{:x}", digest.finalize())
    }
}

pub(crate) fn is_duplicate_column_error(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database_error)
        if database_error.message().contains("duplicate column name"))
}

pub(crate) const MIGRATION_1: &[&str] = &[
    r#"
    create table if not exists nodes (
        node_id text primary key,
        display_name text not null,
        presence text not null,
        sleep_hint text not null,
        last_heartbeat_at text,
        daemon_version text not null,
        active_runtime_count integer not null default 0,
        capabilities_json text not null,
        diagnostics text not null,
        credential_hash text,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists node_enrollments (
        enrollment_id text primary key,
        display_name text not null,
        daemon_version text,
        capabilities_json text not null,
        pairing_code_hash text not null,
        status text not null,
        expires_at text not null,
        claimed_node_id text,
        created_at text not null,
        updated_at text not null,
        approved_at text
    )
    "#,
    r#"
    create table if not exists node_capabilities (
        node_id text not null references nodes(node_id),
        capability_key text not null,
        value_json text not null,
        updated_at text not null,
        primary key (node_id, capability_key)
    )
    "#,
    r#"
    create table if not exists actors (
        actor_key text primary key,
        actor_kind text not null,
        display_name text not null,
        actor_ref_json text not null,
        first_seen_at text not null,
        last_seen_at text not null
    )
    "#,
    r#"
    create table if not exists web_admin (
        id integer primary key check (id = 1),
        password_hash text not null,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists web_sessions (
        session_hash text primary key,
        csrf_hash text not null,
        created_at text not null,
        last_seen_at text not null,
        expires_at text not null
    )
    "#,
    r#"
    create table if not exists security_audit_events (
        audit_event_id text primary key,
        kind text not null,
        node_id text,
        origin text,
        outcome text not null,
        metadata_json text not null,
        happened_at text not null
    )
    "#,
    r#"
    create table if not exists projects (
        project_id text primary key,
        display_name text not null,
        repo_id text,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists project_placements (
        project_placement_id text primary key,
        project_id text references projects(project_id),
        node_id text not null references nodes(node_id),
        display_name text not null,
        workspace_path text not null,
        state text not null,
        resource_badges_json text not null,
        last_validated_at text,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists deleted_workspace_bindings (
        node_id text not null references nodes(node_id),
        workspace_path text not null,
        deleted_at text not null,
        primary key(node_id, workspace_path)
    )
    "#,
    r#"
    create table if not exists session_threads (
        session_thread_id text primary key,
        project_placement_id text not null references project_placements(project_placement_id),
        runtime_session_id text not null unique,
        title text not null,
        state text not null,
        provider text not null,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists runtime_sessions (
        runtime_session_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id),
        provider text not null,
        state text not null,
        resume_supported integer not null,
        provider_resume_ref_json text,
        degraded_reason text,
        last_runtime_step_at text,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists turns (
        turn_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id),
        command_id text not null unique,
        turn_index integer not null,
        state text not null,
        content text not null,
        blocked_approval_id text,
        created_at text not null,
        updated_at text not null,
        completed_at text,
        unique(session_thread_id, turn_index)
    )
    "#,
    r#"
    create table if not exists approvals (
        approval_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id),
        runtime_session_id text,
        turn_id text,
        state text not null,
        request_payload_json text not null,
        response_payload_json text,
        request_command_id text,
        resolve_command_id text,
        requested_event_id text,
        resolved_event_id text,
        created_at text not null,
        updated_at text not null,
        resolved_at text
    )
    "#,
    r#"
    create table if not exists messages (
        message_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id),
        turn_id text,
        role text not null,
        content text not null,
        created_at text not null,
        completed_at text,
        source_event_id text
    )
    "#,
    r#"
    create table if not exists commands (
        command_id text primary key,
        kind text not null,
        state text not null,
        target_node_id text not null,
        session_thread_id text,
        runtime_session_id text,
        project_placement_id text,
        actor_ref_json text not null,
        correlation_id text not null,
        source_refs_json text not null,
        cause_refs_json text not null,
        payload_json text not null,
        result_payload_json text,
        dedupe_key text,
        command_json text not null,
        created_at text not null,
        completed_at text
    )
    "#,
    r#"
    create table if not exists events (
        event_id text primary key,
        scope_key text not null,
        seq integer not null,
        kind text not null,
        node_id text,
        runtime_session_id text,
        session_thread_id text,
        session_projection_seq integer,
        command_id text,
        actor_ref_json text not null,
        scope_ref_json text not null,
        correlation_id text,
        source_refs_json text not null,
        evidence_refs_json text not null,
        cause_refs_json text not null,
        result_refs_json text not null,
        payload_json text not null,
        event_json text not null,
        happened_at text not null,
        unique(scope_key, seq)
    )
    "#,
    r#"
    create unique index if not exists events_session_projection_seq_idx
    on events(session_thread_id, session_projection_seq)
    where session_thread_id is not null and session_projection_seq is not null
    "#,
    r#"
    create table if not exists warning_acknowledgements (
        event_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id),
        actor_ref_json text not null,
        warning_kind text not null,
        command_id text,
        affected_refs_json text not null,
        acknowledged_at text not null
    )
    "#,
];

pub(crate) const MIGRATION_2: &[&str] = &[
    "alter table nodes add column credential_hash text",
    "alter table runtime_sessions add column provider_resume_ref_json text",
    "alter table commands add column actor_ref_json text",
    "alter table commands add column correlation_id text",
    "alter table commands add column source_refs_json text",
    "alter table commands add column cause_refs_json text",
    "alter table commands add column payload_json text",
    "alter table commands add column result_payload_json text",
    "alter table commands add column dedupe_key text",
    "alter table events add column actor_ref_json text",
    "alter table events add column scope_ref_json text",
    "alter table events add column correlation_id text",
    "alter table events add column source_refs_json text",
    "alter table events add column evidence_refs_json text",
    "alter table events add column cause_refs_json text",
    "alter table events add column result_refs_json text",
    "alter table events add column payload_json text",
    "alter table events add column session_projection_seq integer",
    r#"
    update events
    set session_projection_seq = (
        select count(*)
        from events as ordered_events
        where ordered_events.session_thread_id = events.session_thread_id
          and ordered_events.session_thread_id is not null
          and (
            ordered_events.happened_at < events.happened_at
            or (
                ordered_events.happened_at = events.happened_at
                and ordered_events.event_id <= events.event_id
            )
          )
    )
    where session_thread_id is not null
      and session_projection_seq is null
    "#,
];

pub(crate) const MIGRATION_3: &[&str] = &[
    r#"
    create table if not exists core_schema_meta (
        slot text primary key,
        schema_version integer not null,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    insert into core_schema_meta (slot, schema_version, created_at, updated_at)
    values ('0.2.0', 1, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
    "#,
];

pub(crate) const MIGRATION_4: &[&str] = &[
    r#"
    create unique index if not exists project_placements_identity_idx
    on project_placements(node_id, workspace_path)
    "#,
    r#"
    create unique index if not exists runtime_sessions_session_thread_idx
    on runtime_sessions(session_thread_id)
    "#,
    r#"
    create unique index if not exists approvals_turn_idx
    on approvals(turn_id)
    where turn_id is not null
    "#,
    r#"
    create unique index if not exists messages_source_event_idx
    on messages(source_event_id)
    where source_event_id is not null
    "#,
];

pub(crate) const MIGRATION_5: &[&str] = &[
    "alter table events add column projection_state text not null default 'pending'",
    "alter table events add column projection_attempts integer not null default 0",
    "alter table events add column projected_at text",
    r#"
    create index if not exists events_projection_pending_idx
    on events(projection_state, happened_at)
    where projection_state <> 'projected'
    "#,
];

pub(crate) const MIGRATION_6: &[&str] = &[
    r#"
    create table if not exists event_publication_outbox (
        event_id text primary key references events(event_id) on delete cascade,
        event_json text not null,
        attempts integer not null default 0,
        enqueued_at text not null,
        published_at text
    )
    "#,
    r#"
    create index if not exists event_publication_outbox_pending_idx
    on event_publication_outbox(enqueued_at, event_id)
    where published_at is null
    "#,
];

pub(crate) const MIGRATION_7: &[&str] = &[
    r#"
    create table if not exists command_dispatch_outbox (
        command_id text primary key references commands(command_id) on delete cascade,
        target_node_id text not null,
        command_json text not null,
        attempts integer not null default 0,
        enqueued_at text not null,
        last_attempt_at text
    )
    "#,
    r#"
    create index if not exists command_dispatch_outbox_pending_idx
    on command_dispatch_outbox(target_node_id, enqueued_at, command_id)
    "#,
    r#"
    insert into command_dispatch_outbox (
        command_id, target_node_id, command_json, enqueued_at
    )
    select command_id, target_node_id, command_json, created_at
    from commands
    where state in ('recorded', 'pending_dispatch', 'dispatched', 'acknowledged')
      and not exists (
          select 1 from command_dispatch_outbox outbox
          where outbox.command_id = commands.command_id
      )
    "#,
];

pub(crate) const MIGRATION_8: &[&str] = &[
    r#"
    create table if not exists scheduled_messages (
        scheduled_message_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id) on delete cascade,
        content text not null,
        due_at text not null,
        timezone text not null,
        state text not null check (state in ('scheduled', 'sending', 'sent', 'failed', 'cancelled')),
        created_at text not null,
        updated_at text not null,
        sending_at text,
        sent_at text,
        cancelled_at text,
        command_id text references commands(command_id),
        turn_id text references turns(turn_id),
        failure_code text,
        failure_message text
    )
    "#,
    r#"
    create index if not exists scheduled_messages_due_idx
    on scheduled_messages(state, due_at, scheduled_message_id)
    where state = 'scheduled'
    "#,
    r#"
    create index if not exists scheduled_messages_session_idx
    on scheduled_messages(session_thread_id, due_at desc, created_at desc)
    "#,
];

pub(crate) const MIGRATION_9: &[&str] = &[
    r#"
    create table if not exists jobs (
        job_id text primary key,
        project_placement_id text not null references project_placements(project_placement_id) on delete cascade,
        name text not null,
        prompt text not null,
        provider text not null,
        schedule_json text,
        timezone text not null,
        enabled integer not null default 0,
        overlap_policy text not null check (overlap_policy = 'skip'),
        continue_after_error integer not null default 0,
        next_run_at text,
        paused_reason text,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists job_runs (
        job_run_id text primary key,
        job_id text not null references jobs(job_id) on delete cascade,
        trigger_kind text not null check (trigger_kind in ('manual', 'scheduled')),
        state text not null check (state in ('queued', 'starting', 'running', 'succeeded', 'failed', 'cancelled', 'timed_out', 'skipped')),
        scheduled_for text,
        queued_at text not null,
        started_at text,
        finished_at text,
        session_thread_id text references session_threads(session_thread_id),
        runtime_session_id text references runtime_sessions(runtime_session_id),
        command_id text references commands(command_id),
        turn_id text references turns(turn_id),
        summary text,
        terminal_code text,
        terminal_message text,
        config_snapshot_json text not null,
        force integer not null default 0
    )
    "#,
    r#"
    create unique index if not exists job_runs_scheduled_occurrence_idx
    on job_runs(job_id, scheduled_for)
    where trigger_kind = 'scheduled' and scheduled_for is not null
    "#,
    r#"
    create index if not exists job_runs_active_idx
    on job_runs(job_id, state, queued_at)
    where state in ('queued', 'starting', 'running')
    "#,
    r#"
    create unique index if not exists job_runs_one_active_idx
    on job_runs(job_id)
    where state in ('queued', 'starting', 'running')
    "#,
    r#"
    create index if not exists jobs_due_idx
    on jobs(enabled, next_run_at, job_id)
    where enabled = 1 and next_run_at is not null
    "#,
    r#"
    create table if not exists provider_quota_snapshots (
        provider text primary key,
        five_hour_remaining_percent integer,
        weekly_remaining_percent integer,
        observed_at text not null,
        reliable integer not null default 0,
        source text not null
    )
    "#,
];

pub(crate) const MIGRATION_10: &[&str] = &[
    r#"
    create table if not exists deductions (
        deduction_id text primary key,
        session_thread_id text not null references session_threads(session_thread_id) on delete cascade,
        scope_ref_json text not null,
        question text not null,
        state text not null check (state in ('requested', 'running', 'completed', 'invalid', 'failed', 'cancelled')),
        command_id text not null unique references commands(command_id),
        evidence_snapshot_hash text not null,
        input_package_json text not null,
        block_json text,
        raw_fallback text,
        raw_truncated integer not null default 0,
        error_code text,
        error_message text,
        artifact_id text,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create index if not exists deductions_session_idx
    on deductions(session_thread_id, created_at desc, deduction_id)
    "#,
    r#"
    create unique index if not exists deductions_one_active_idx
    on deductions(session_thread_id)
    where state in ('requested', 'running')
    "#,
    r#"
    create table if not exists causality_narratives (
        artifact_id text primary key,
        deduction_id text not null unique references deductions(deduction_id) on delete cascade,
        session_thread_id text not null references session_threads(session_thread_id) on delete cascade,
        current_version integer not null,
        created_at text not null,
        updated_at text not null
    )
    "#,
    r#"
    create table if not exists causality_narrative_versions (
        artifact_id text not null references causality_narratives(artifact_id) on delete cascade,
        version integer not null,
        block_json text not null,
        provenance_json text not null,
        created_at text not null,
        primary key (artifact_id, version)
    )
    "#,
];

pub(crate) const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        statements: MIGRATION_1,
        ignore_duplicate_columns: false,
    },
    Migration {
        version: 2,
        statements: MIGRATION_2,
        ignore_duplicate_columns: true,
    },
    Migration {
        version: 3,
        statements: MIGRATION_3,
        ignore_duplicate_columns: false,
    },
    Migration {
        version: 4,
        statements: MIGRATION_4,
        ignore_duplicate_columns: false,
    },
    Migration {
        version: 5,
        statements: MIGRATION_5,
        ignore_duplicate_columns: true,
    },
    Migration {
        version: 6,
        statements: MIGRATION_6,
        ignore_duplicate_columns: false,
    },
    Migration {
        version: 7,
        statements: MIGRATION_7,
        ignore_duplicate_columns: false,
    },
    Migration {
        version: 8,
        statements: MIGRATION_8,
        ignore_duplicate_columns: false,
    },
    Migration {
        version: 9,
        statements: MIGRATION_9,
        ignore_duplicate_columns: false,
    },
    Migration {
        version: 10,
        statements: MIGRATION_10,
        ignore_duplicate_columns: false,
    },
];
