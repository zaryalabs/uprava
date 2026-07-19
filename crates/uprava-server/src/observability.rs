use std::sync::atomic::{AtomicU64, Ordering};

/// Low-cardinality process counters exposed by the Core metrics endpoint.
#[derive(Default)]
pub(crate) struct CoreMetrics {
    pub(crate) accepted_events: AtomicU64,
    pub(crate) command_results: AtomicU64,
    pub(crate) auth_failures: AtomicU64,
    pub(crate) control_queue_rejections: AtomicU64,
    pub(crate) requests: AtomicU64,
    pub(crate) request_errors: AtomicU64,
    pub(crate) request_duration_micros: AtomicU64,
    pub(crate) requests_in_flight: AtomicU64,
    pub(crate) public_rate_rejections: AtomicU64,
    pub(crate) provider_truncations: AtomicU64,
    pub(crate) pty_opened: AtomicU64,
    pub(crate) pty_terminal_states: AtomicU64,
    pub(crate) tool_registry_searches: AtomicU64,
    pub(crate) tool_execution_requests: AtomicU64,
    pub(crate) tool_execution_failures: AtomicU64,
    pub(crate) tool_policy_denials: AtomicU64,
    pub(crate) tool_dependency_errors: AtomicU64,
    pub(crate) mcp_leases_issued: AtomicU64,
    pub(crate) mcp_lease_rejections: AtomicU64,
}

impl CoreMetrics {
    pub(crate) fn render(&self) -> String {
        format!(
            "# HELP uprava_core_events_accepted_total Accepted event envelopes.\n# TYPE uprava_core_events_accepted_total counter\nuprava_core_events_accepted_total {}\n# HELP uprava_core_command_results_total Command results received from Nodes.\n# TYPE uprava_core_command_results_total counter\nuprava_core_command_results_total {}\n# HELP uprava_core_auth_failures_total Rejected authentication attempts.\n# TYPE uprava_core_auth_failures_total counter\nuprava_core_auth_failures_total {}\n# HELP uprava_core_control_queue_rejections_total Control frames rejected by a saturated or closed Node queue.\n# TYPE uprava_core_control_queue_rejections_total counter\nuprava_core_control_queue_rejections_total {}\n# HELP uprava_core_requests_total HTTP requests.\n# TYPE uprava_core_requests_total counter\nuprava_core_requests_total {}\n# HELP uprava_core_request_errors_total HTTP error responses.\n# TYPE uprava_core_request_errors_total counter\nuprava_core_request_errors_total {}\n# HELP uprava_core_request_duration_seconds_sum Total HTTP request duration.\n# TYPE uprava_core_request_duration_seconds_sum counter\nuprava_core_request_duration_seconds_sum {:.6}\n# HELP uprava_core_requests_in_flight Current HTTP requests.\n# TYPE uprava_core_requests_in_flight gauge\nuprava_core_requests_in_flight {}\n# HELP uprava_core_public_rate_rejections_total Public ingress quota rejections.\n# TYPE uprava_core_public_rate_rejections_total counter\nuprava_core_public_rate_rejections_total {}\n# HELP uprava_core_provider_truncations_total Provider output truncation events.\n# TYPE uprava_core_provider_truncations_total counter\nuprava_core_provider_truncations_total {}\n# HELP uprava_core_pty_opened_total Opened PTYs.\n# TYPE uprava_core_pty_opened_total counter\nuprava_core_pty_opened_total {}\n# HELP uprava_core_pty_terminal_states_total Terminal PTY state notifications.\n# TYPE uprava_core_pty_terminal_states_total counter\nuprava_core_pty_terminal_states_total {}\n# HELP uprava_core_tool_registry_searches_total Progressive tool searches.\n# TYPE uprava_core_tool_registry_searches_total counter\nuprava_core_tool_registry_searches_total {}\n# HELP uprava_core_tool_execution_requests_total Tool Execute requests.\n# TYPE uprava_core_tool_execution_requests_total counter\nuprava_core_tool_execution_requests_total {}\n# HELP uprava_core_tool_execution_failures_total Terminal tool execution failures.\n# TYPE uprava_core_tool_execution_failures_total counter\nuprava_core_tool_execution_failures_total {}\n# HELP uprava_core_tool_policy_denials_total Tool policy denials.\n# TYPE uprava_core_tool_policy_denials_total counter\nuprava_core_tool_policy_denials_total {}\n# HELP uprava_core_tool_dependency_errors_total Failed or degraded ToolHive dependency reports.\n# TYPE uprava_core_tool_dependency_errors_total counter\nuprava_core_tool_dependency_errors_total {}\n# HELP uprava_core_mcp_leases_issued_total Session-scoped MCP leases issued.\n# TYPE uprava_core_mcp_leases_issued_total counter\nuprava_core_mcp_leases_issued_total {}\n# HELP uprava_core_mcp_lease_rejections_total Rejected MCP access leases.\n# TYPE uprava_core_mcp_lease_rejections_total counter\nuprava_core_mcp_lease_rejections_total {}\n# HELP uprava_core_log_records_dropped_total Log records dropped by the bounded writer.\n# TYPE uprava_core_log_records_dropped_total counter\nuprava_core_log_records_dropped_total {}\n# HELP uprava_core_otlp_export_failures_total Failed OTLP batches or initialization attempts.\n# TYPE uprava_core_otlp_export_failures_total counter\nuprava_core_otlp_export_failures_total {}\n",
            self.accepted_events.load(Ordering::Relaxed),
            self.command_results.load(Ordering::Relaxed),
            self.auth_failures.load(Ordering::Relaxed),
            self.control_queue_rejections.load(Ordering::Relaxed),
            self.requests.load(Ordering::Relaxed),
            self.request_errors.load(Ordering::Relaxed),
            self.request_duration_micros.load(Ordering::Relaxed) as f64 / 1_000_000.0,
            self.requests_in_flight.load(Ordering::Relaxed),
            self.public_rate_rejections.load(Ordering::Relaxed),
            self.provider_truncations.load(Ordering::Relaxed),
            self.pty_opened.load(Ordering::Relaxed),
            self.pty_terminal_states.load(Ordering::Relaxed),
            self.tool_registry_searches.load(Ordering::Relaxed),
            self.tool_execution_requests.load(Ordering::Relaxed),
            self.tool_execution_failures.load(Ordering::Relaxed),
            self.tool_policy_denials.load(Ordering::Relaxed),
            self.tool_dependency_errors.load(Ordering::Relaxed),
            self.mcp_leases_issued.load(Ordering::Relaxed),
            self.mcp_lease_rejections.load(Ordering::Relaxed),
            uprava_logging::dropped_log_records(),
            uprava_logging::otlp_export_failures(),
        )
    }
}
