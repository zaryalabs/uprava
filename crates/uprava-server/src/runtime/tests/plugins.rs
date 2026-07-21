use uprava_protocol::{
    PluginCompatibilityState, PluginContribution, PluginDesiredState, PluginEffectiveState,
    PluginId,
};

use super::*;

#[tokio::test]
async fn bundled_plugins_should_bootstrap_idempotently() {
    let state = test_state().await;

    bootstrap_bundled_plugins(&state)
        .await
        .expect("second bootstrap succeeds");
    let plugins = list_plugins(&state).await.expect("plugins load");

    assert_eq!(plugins.items.len(), 2);
    let latest_migration: i64 = sqlx::query_scalar("select max(version) from schema_migrations")
        .fetch_one(&state.pool)
        .await
        .expect("migration version loads");
    assert_eq!(latest_migration, 13);
}

#[tokio::test]
async fn markdown_plugin_should_bootstrap_enabled() {
    let state = test_state().await;
    let plugin = load_plugin_installation(&state, &PluginId::from("uprava.markdown"))
        .await
        .expect("Markdown plugin loads");

    assert_eq!(plugin.desired_state, PluginDesiredState::Enabled);
    assert_eq!(plugin.effective_state, PluginEffectiveState::Active);
    assert_eq!(
        plugin.compatibility.state,
        PluginCompatibilityState::Compatible
    );
    assert_eq!(plugin.granted_permissions, ["visual.renderer.contribute"]);
}

#[tokio::test]
async fn dark_theme_should_bootstrap_disabled() {
    let state = test_state().await;
    let plugin = load_plugin_installation(&state, &PluginId::from("uprava.theme-dark"))
        .await
        .expect("theme plugin loads");

    assert_eq!(plugin.desired_state, PluginDesiredState::Disabled);
    assert_eq!(plugin.effective_state, PluginEffectiveState::Disabled);
}

#[tokio::test]
async fn plugin_enable_and_disable_should_change_effective_snapshot() {
    let state = test_state().await;
    let plugin_id = PluginId::from("uprava.theme-dark");

    set_plugin_desired_state(&state, &plugin_id, PluginDesiredState::Enabled)
        .await
        .expect("plugin enables");
    let enabled = effective_plugin_snapshot(&state)
        .await
        .expect("snapshot loads");
    assert!(matches!(
        enabled.contributions.as_slice(),
        [
            PluginContribution::VisualRenderer { .. },
            PluginContribution::UiTheme { .. }
        ]
    ));

    set_plugin_desired_state(&state, &plugin_id, PluginDesiredState::Disabled)
        .await
        .expect("plugin disables");
    let disabled = effective_plugin_snapshot(&state)
        .await
        .expect("snapshot loads");
    assert!(matches!(
        disabled.contributions.as_slice(),
        [PluginContribution::VisualRenderer { .. }]
    ));
}

#[tokio::test]
async fn plugin_desired_state_should_survive_core_restart() {
    let pool = memory_pool().await;
    let config = test_config(86_400);
    let state = AppState::new(config.clone(), pool.clone())
        .await
        .expect("state starts");
    let plugin_id = PluginId::from("uprava.theme-dark");
    set_plugin_desired_state(&state, &plugin_id, PluginDesiredState::Enabled)
        .await
        .expect("plugin enables");
    drop(state);

    let restarted = AppState::new(config, pool).await.expect("state restarts");
    let plugin = load_plugin_installation(&restarted, &plugin_id)
        .await
        .expect("plugin loads");

    assert_eq!(plugin.desired_state, PluginDesiredState::Enabled);
    assert_eq!(plugin.effective_state, PluginEffectiveState::Active);
}

#[tokio::test]
async fn plugin_lifecycle_should_write_bounded_audit_events() {
    let state = test_state().await;
    let plugin_id = PluginId::from("uprava.theme-dark");

    set_plugin_desired_state(&state, &plugin_id, PluginDesiredState::Enabled)
        .await
        .expect("plugin enables");
    let metadata: String = sqlx::query_scalar(
        "select metadata_json from security_audit_events where kind = 'plugin.enabled' order by happened_at desc limit 1",
    )
    .fetch_one(&state.pool)
    .await
    .expect("audit event loads");

    assert!(metadata.contains("uprava.theme-dark"));
    assert!(!metadata.contains("semantic_tokens"));
}
