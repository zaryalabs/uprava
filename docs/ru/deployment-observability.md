# Uprava Deployment and Observability

Статус: `working-position`

Этот документ фиксирует рабочую позицию по production-like deployment для
Uprava на сервере Zarya: **Core stack в Docker, Node Daemon на host через
systemd**. Цель - получить удобный деплой без потери прозрачности: Docker
сервисы остаются видимыми через обычные контейнерные логи, а bare-metal daemon
становится видимым через OpenTelemetry logs and metrics.

CI/CD, release artifacts, `.env.release`, server Makefile behavior and systemd
activation rules live in [`deployment.md`](deployment.md). This document owns
observability data paths, minimum metrics, dashboards, alerts and telemetry
failure behavior.

## Короткое решение

Production shape для `zarya-main`:

```text
Traefik / platform network
        |
        v
Uprava Core stack in Docker
  uprava-core  - Rust Core Backend, API, auth, state, event log
  uprava-web   - built Vite Web Control Panel, same public origin as Core
  versioned core.sqlite - product-owned state slot 0.1.8 or 0.2.0

Host systemd
  uprava-node.service - Rust Node Daemon near real files, PTY, Codex, tools

Shared observability
  Promtail -> Loki        for Docker stdout/stderr logs
  OTLP -> Collector       for Node logs and Core/Node app metrics
  Collector -> Prometheus for metrics scrape
  Collector -> Loki       for Node OTLP logs
  Grafana                for dashboards and alerts
```

Core and Web belong to the product Docker stack under `/opt/apps/uprava`.
Shared Grafana, Prometheus, Loki, Promtail, OpenTelemetry Collector and Traefik
stay under `/opt/infra`.

Node Daemon is not run in the production Docker stack by default. It is a host
service because it owns local workspace access, PTY/process lifecycle, provider
execution, local tools and host credentials. The Docker `node` service remains
useful for local smoke tests and synthetic workspaces.

## Goals

- Make Core/Web deployment predictable through the existing product stack model.
- Keep Node Daemon close to the real host resources it controls.
- Preserve centralized logs for both Docker and bare-metal processes.
- Give Core and Node a common app-metrics path.
- Avoid exposing unauthenticated OTLP receivers to the public internet.
- Keep observability non-blocking: telemetry failure must not stop agent work.
- Keep metrics low-cardinality and logs/event ids detailed enough for
  investigation.

## Non-goals

- Do not move Node Daemon into Docker just for operational uniformity.
- Do not make Core scrape host files or processes directly.
- Do not add a product-local observability stack for Uprava.
- Do not use Prometheus labels for high-cardinality ids like `session_id`,
  `command_id`, file paths or raw correlation ids.
- Do not rely on Portainer/manual server edits as the source of truth.

## Deployment Topology

### Docker Core Stack

The product stack should contain the browser-facing part of Uprava:

- `uprava-core`: Rust Core Backend.
- `uprava-web`: built Web Control Panel.
- persistent Core state volume or bind mount.
- attachment to the external `platform` Docker network for Traefik.
- no public host port binding except local smoke/debug ports when explicitly
  enabled.

Near-term implementation may keep Core and Web as separate containers, matching
the current local Compose shape. The target public contract should still be one
browser origin:

```text
https://uprava.zrya.io/          -> Web UI
https://uprava.zrya.io/api/v1    -> Core API
https://uprava.zrya.io/api/v1/.../stream or /control -> Core event/control paths
```

This avoids production CORS complexity and lets Core use secure cookies with
`UPRAVA_COOKIE_SECURE=true`.

The Web build should support same-origin API configuration, preferably with a
relative base such as `/api/v1` in production. The current local default
`http://127.0.0.1:8080/api/v1` should remain a development default only.

Core state remains product-owned. SQLite acceptable для current single-server
controlled deployment, но выбранный versioned state slot должен входить в
backup and restore procedures. Slots 0.1.8 and 0.2.0 и matching configuration
остаются раздельными по контракту
[`deployment.md`](deployment.md).

### Bare-metal Node Daemon

The Node Daemon should run as a systemd unit on the host:

```text
/etc/systemd/system/uprava-node.service
/etc/uprava/node.env -> /etc/uprava/releases/<active-version>/node.env
/var/lib/uprava/
/var/lib/uprava-node/0.1.8/node.json
/var/lib/uprava-node/0.2.0/node.sqlite
/var/log/uprava-node/ optional local fallback logs
```

Default service posture:

- run as the dedicated Unix user `uprava`, not root;
- grant workspace access explicitly through filesystem ownership/group policy;
- configure `UPRAVA_NODE_WORKSPACES` with explicit allowed roots;
- configure `UPRAVA_CORE_URL=https://uprava.zrya.io`;
- store Node local state outside the repository;
- restart on failure with a short delay;
- keep journal logs as a local fallback, while central log shipping goes through
  OTLP.

If a specific provider requires an existing operator account, that should be an
explicit deployment decision. It should not silently become "run daemon as root"
because that is convenient.

Minimal unit shape:

```ini
[Unit]
Description=Uprava Node Daemon
After=network-online.target
Wants=network-online.target

[Service]
User=uprava
Group=uprava
EnvironmentFile=/etc/uprava/node.env
ExecStart=/opt/apps/uprava/current/uprava-node
Restart=on-failure
RestartSec=5s
WorkingDirectory=/var/lib/uprava

[Install]
WantedBy=multi-user.target
```

Host-local `journalctl -u uprava-node` remains useful during incidents, but it
is not the central observability path.

## Observability Data Paths

### Logs

Docker services:

```text
uprava-core stdout/stderr
uprava-web stdout/stderr
        |
        v
Promtail docker_sd_configs
        |
        v
Loki
```

Bare-metal Node:

```text
uprava-node tracing logs
        |
        v
OTLP logs exporter
        |
        v
OpenTelemetry Collector
        |
        v
Loki
```

Node may also write local file or journald logs for break-glass diagnosis, but
the production dashboard should read central logs from Loki.

Core currently accepts browser client logs through `POST /api/v1/client/logs`
and writes JSONL. For production, accepted client logs should also become Core
structured log events so they reach Loki through the Docker log path. The JSONL
file may remain a local diagnostic artifact, but should not be the only central
path for browser errors.

Required log attributes:

- `service.name`: `uprava-core`, `uprava-web` or `uprava-node`;
- `service.namespace`: `zarya`;
- `deployment.environment`: `zarya-main`;
- `deployment.profile`: `controlled_dev` until a stricter profile exists;
- `service.instance.id`: container id or daemon installation id;
- `node.id` when known;
- `correlation_id` for request/command/runtime flows;
- event kind, command kind, runtime state and result fields where relevant.

Do not log:

- bearer tokens;
- node credentials;
- pairing codes;
- session cookies or CSRF values;
- full prompt/command payloads by default;
- full file contents;
- provider secrets or local env dumps.

### Metrics

Both Core and Node should send app metrics through OTLP to the shared collector.
Prometheus should keep scraping the collector's Prometheus exporter as it does
today.

```text
uprava-core OTLP metrics
uprava-node OTLP metrics
        |
        v
OpenTelemetry Collector
        |
        v
Prometheus exporter :8889
        |
        v
Prometheus scrape
        |
        v
Grafana
```

The collector should be reachable from containers on the `platform` network and
from the host over loopback only:

```text
containers: http://otel-collector:4318
host daemon: http://127.0.0.1:4318
```

Do not publish OTLP ports publicly without TLS and authentication. For future
remote nodes, either add an authenticated HTTPS OTLP endpoint or route telemetry
through a secured Core/relay path.

### Traces

Full distributed tracing is not required for the first deployment cut.
Correlation ids, event log, structured logs and basic metrics are enough.

The protocol should not block future traces. The same `correlation_id` should
continue to propagate through HTTP requests, command envelopes, Node events and
runtime flows.

## Minimum Metrics

Metrics must be useful for operations without exploding label cardinality.

### Core Metrics

HTTP:

- `uprava_http_requests_total` by `route`, `method`, `status_class`;
- `uprava_http_request_duration_seconds` by `route`, `method`;
- `uprava_http_in_flight_requests` by `route`.

Auth and client:

- auth setup/login/logout/failure counters;
- CSRF/origin rejection counters;
- accepted browser client log counters by `level` and `source`.

Node/control plane:

- node enrollment created/approved/claimed/rejected counters;
- heartbeat accepted/rejected counters;
- `uprava_node_last_heartbeat_age_seconds` by `node_id` only if node count is
  small; otherwise expose per-node state through UI/API and keep metrics
  aggregate;
- control channel connect/disconnect counters;
- active control channel gauge.

Runtime/commands/events:

- command recorded/dispatched/result counters by `command_kind` and `status`;
- command dispatch latency histogram by `command_kind`;
- events accepted counter by `event_kind`;
- stream gap counter;
- active runtime gauge by `runtime_state`;
- runtime start/stop/resume/error counters;
- turn started/completed/interrupted/error counters.

Storage/process:

- Core SQLite operation error counter;
- Core process uptime gauge;
- build/version info gauge with labels `version`, `git_sha` when available.

### Node Metrics

Heartbeat/control:

- heartbeat attempt/success/failure counters;
- heartbeat latency histogram;
- control channel connection state gauge;
- control frame sent/received/error counters.

Runtime/provider:

- active runtime gauge by `provider` and `runtime_state`;
- provider command duration histogram by `command_kind` and `provider`;
- provider command result counters;
- approval requested/resolved counters;
- runtime event outbox size gauge;
- event outbox dropped counter.

Workspace/terminal:

- workspace validation counters by result;
- workspace command run counters by command family and result;
- workspace command duration histogram;
- terminal open/close counters;
- active terminal gauge.

Telemetry health:

- OTLP export failure/drop counters;
- local log fallback write failures;
- process uptime gauge;
- build/version info gauge.

## Cardinality Rules

Allowed metric labels:

- service;
- route template, not raw URL;
- method;
- status class or stable status code;
- command kind;
- event kind;
- runtime state;
- provider name;
- result/status;
- deployment target.

Avoid metric labels:

- `session_id`;
- `runtime_session_id`;
- `command_id`;
- `event_id`;
- `correlation_id`;
- file path;
- prompt text;
- user input;
- dynamic error message.

Those ids belong in logs, event storage and trace metadata, not in Prometheus
time-series labels.

## Required Infra Changes

In `infra/observability`:

1. Keep Promtail Docker log collection for Core/Web.
2. Expose OTLP receiver ports to the host on loopback only:

```yaml
ports:
  - "127.0.0.1:4317:4317"
  - "127.0.0.1:4318:4318"
```

3. Keep the collector on the external `platform` network with alias
   `otel-collector` for Docker services.
4. Keep the metrics pipeline from OTLP to the Prometheus exporter.
5. Add a logs pipeline from OTLP to Loki using the supported collector/Loki
   path for the pinned collector image.
6. Add collector health and export-failure visibility to Grafana.

The exact collector exporter should be pinned during implementation. Uprava
should not carry a second collector or product-local Loki just for Node logs.

## Required Product Changes

### `uprava-logging`

Extend the shared logging crate so services can configure:

- text or JSON stdout logs;
- local file fallback;
- OTLP log export for Node;
- OTLP metric export for Core and Node;
- resource attributes shared by logs and metrics.

Telemetry initialization should be non-blocking. Exporter failure should emit a
warning and increment/drop telemetry counters, not fail Core startup or stop
Node runtime work.

### Core

Core should:

- emit production-readable structured logs to stdout;
- emit accepted browser client logs as structured tracing events;
- expose/emit the Core metrics listed above;
- keep propagating `x-correlation-id` into command and event flows;
- support same-origin production web/API routing;
- keep `/api/v1/health` as the container healthcheck.

### Node

Node should:

- keep local journald/file logging as fallback;
- export logs through OTLP when `UPRAVA_OTEL_LOGS_ENABLED=true`;
- export metrics through OTLP when `UPRAVA_OTEL_METRICS_ENABLED=true`;
- include daemon installation id and node id as resource/log attributes when
  known;
- report telemetry exporter degradation without breaking heartbeat/control
  loops.

Suggested environment variables:

```text
UPRAVA_OTEL_ENDPOINT=http://127.0.0.1:4318
UPRAVA_OTEL_PROTOCOL=http/protobuf
UPRAVA_OTEL_METRICS_ENABLED=true
UPRAVA_OTEL_LOGS_ENABLED=true
UPRAVA_OTEL_SERVICE_NAMESPACE=zarya
UPRAVA_DEPLOYMENT_TARGET=zarya-main
```

Core containers can use:

```text
UPRAVA_OTEL_ENDPOINT=http://otel-collector:4318
UPRAVA_OTEL_METRICS_ENABLED=true
UPRAVA_OTEL_LOGS_ENABLED=false
```

Core logs can remain Docker-collected initially; enabling Core OTLP logs later
is optional.

## Dashboards

Minimum Grafana dashboard set:

1. **Uprava Overview**
   - Core up/down;
   - Web up/down;
   - Node online/stale/offline counts;
   - active runtimes;
   - command error rate;
   - recent runtime errors;
   - logs panel filtered by `service.name`.

2. **Uprava Core**
   - HTTP request rate, latency and error rate;
   - auth/security rejection counters;
   - command dispatch rate/result;
   - event append rate/gaps;
   - SQLite/database errors.

3. **Uprava Node**
   - heartbeat success/failure and latency;
   - control channel state;
   - active runtimes and terminals;
   - provider command duration/result;
   - event outbox size and drops;
   - Node logs by level.

4. **Telemetry Health**
   - collector health;
   - OTLP export failures;
   - Prometheus scrape health;
   - Loki ingestion/query errors if available.

## Alerts

Initial alerts should stay operational and low-noise:

- Core healthcheck failing for 2-5 minutes.
- Web route unavailable.
- Node heartbeat stale/offline beyond expected heartbeat window.
- Control channel repeatedly failing while commands are pending.
- Runtime error rate above threshold.
- Event outbox drops > 0.
- OTLP export failure sustained for Node.
- Prometheus cannot scrape collector.
- Loki ingestion unavailable.

Security alerts can be added after baseline:

- repeated login failures;
- repeated CSRF/origin rejections;
- node credential revoked/rejected;
- unexpected node re-enrollment loop.

## Failure Behavior

Telemetry failure:

- Core and Node continue running.
- Node continues heartbeat/control loops.
- Logs stay available locally through journald/file fallback.
- Export failures are visible as counters and warnings when possible.

Collector down:

- Core/Web Docker logs still go through Promtail if Loki is healthy.
- Node central logs may be delayed or dropped depending on exporter buffer.
- Node should not block runtime operations.

Loki down:

- Promtail/collector may retry according to their own buffering.
- Grafana log panels degrade, but Core/Node continue.

Prometheus down:

- Metrics dashboards and alerts degrade.
- Logs and product runtime continue.

Core down:

- Node retries heartbeat and control connection.
- Node local state remains intact.
- systemd should not restart Node solely because Core is unavailable.

Reset или telemetry incident не должны удалять retained release-family
state/config slots. В частности, reset 0.2.0 никогда не удаляет state или
configuration 0.1.8.

## Rollout Plan

1. **Ops skeleton**
   - Add production `ops/` assets for Core/Web Docker stack.
   - Add systemd unit and env-file template for Node.
   - Configure same-origin Traefik routes.
   - Add backup/restore notes for Core SQLite.

2. **Metrics first**
   - Add shared metrics initialization to `uprava-logging` or a small telemetry
     crate.
   - Instrument Core HTTP, command/event and runtime paths.
   - Instrument Node heartbeat/control/runtime/provider paths.
   - Extend the collector metrics path only if needed.

3. **Node OTLP logs**
   - Add OTLP log export for Node.
   - Extend `infra/observability` with the logs pipeline to Loki.
   - Keep journald/file fallback.

4. **Dashboards and alerts**
   - Add Grafana dashboard provisioning.
   - Add basic alert rules after metric names stabilize.
   - Add smoke checks for collector ingest and dashboard queries where practical.

5. **Hardening**
   - Pin observability images instead of `latest`.
   - Decide whether remote nodes need authenticated OTLP over HTTPS.
   - Add telemetry drop/backpressure tests.
   - Add deployment smoke checks that verify Core, Web, Node heartbeat, one
     app metric and one Node log in the central stack.

## Open Questions

- Should production Web remain a separate container, or should Core serve the
  built static assets after the local profile stabilizes?
- Should Core eventually emit OTLP logs too, or is Docker log collection enough
  for the controlled single-server deployment?
- What is the first production domain for Uprava: `uprava.zrya.io` or another
  host?
- Which Unix user should own real workspaces used by the host Node Daemon?
- Do future remote nodes send OTLP directly to observability, through Core, or
  through a dedicated secured telemetry gateway?
